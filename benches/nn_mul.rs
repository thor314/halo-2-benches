#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use halo2_proofs::{
  arithmetic::CurveAffine,
  circuit::{Chip, SimpleFloorPlanner, Value},
  pasta::{
    group::ff::{Field, FromUniformBytes},
    vesta, Fp,
  },
  plonk::{
    create_proof, keygen_pk, keygen_vk, Advice, Circuit, Column, ConstraintSystem, Fixed, Instance,
    ProvingKey, VerifyingKey,
  },
  poly::commitment::Params,
  transcript::{Challenge255, Transcript},
};
use halo_2_benches::gadgets::scalar_mul::*;

type VestaAffine = vesta::Affine;

/// returning a*b
#[derive(Default, Clone)]
pub struct NNMulCircuit<F: Field> {
  pub a: Value<F>,
  pub b: Value<F>,
}

impl<F: Field> Circuit<F> for NNMulCircuit<F> {
  // the chip needs to be configured
  // field choice for the Circuit, see below
  // can have Circuit config overlap with Chip config since only one Chip
  type Config = ScalarMulConfig;
  // algorithm to plan table layout, using the default here
  type FloorPlanner = SimpleFloorPlanner;

  // typically just default
  fn without_witnesses(&self) -> Self { Self::default() }

  // describe exact gate/column arrangement
  fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
    // used for IO; have a fan-in 2 circuit gate, so need 2 advice cols
    let advice = [meta.advice_column(), meta.advice_column()];
    // store public inputs in Instance columns
    let instance = meta.instance_column();
    // for loading a constant
    let constant = meta.fixed_column();
    // return the column configuration
    Self::Config::configure(meta, advice, instance, constant)
  }

  // Create the circuit WRT the constraint system
  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl halo2_proofs::circuit::Layouter<F>,
  ) -> Result<(), halo2_proofs::plonk::Error> {
    // load any used arithmetic chips; see below for the construction of our chip
    let field_chip = ScalarMulChip::<F>::new(config);

    // Load {private, constant} values into the circuit
    let a = field_chip.load_private(layouter.namespace(|| "load a"), self.a)?;
    let b = field_chip.load_private(layouter.namespace(|| "load b"), self.b)?;
    // Finally, tell the circuit how to use our Chip
    let c = field_chip.mul(layouter.namespace(|| "a * b"), a.clone(), a)?;

    // and "return" the result as a public input to the circuit
    field_chip.expose_public(layouter.namespace(|| "expose result"), c, 0)
  }
}

pub struct Workbench {
  name:            String,
  params:          Params<VestaAffine>,
  pk:              ProvingKey<VestaAffine>,
  vk:              VerifyingKey<VestaAffine>,
  circuit:         NNMulCircuit<Fp>,
  expected_output: Fp,
  rng:             rand::rngs::OsRng,
}

pub fn workbench() -> Workbench {
  // ANCHOR: test-circuit
  // 2^k is the number of rows in our circuit
  let k = 4;
  // Instantiate the circuit with the private inputs.
  let (circuit, expected_output) = {
    let a = Fp::from(2);
    let b = Fp::from(3);
    let c = a.square() * b.square();
    let (a, b) = (Value::known(a), Value::known(b));
    (NNMulCircuit { a, b }, c)
  };

  // Initialize the proving key
  let params = Params::new(k);
  let vk = keygen_vk(&params, &circuit).expect("keygen_vk should not fail");
  let pk = keygen_pk(&params, vk.clone(), &circuit).expect("keygen_pk should not fail");

  // Arrange the public input. We expose the multiplication result in row 0
  // of the instance column, so we position it there in our public inputs.
  // let mut public_inputs = vec![c];

  // // Given the correct public input, our circuit will verify.
  // let prover = MockProver::run(k, &my_circuit, vec![public_inputs.clone()]).unwrap();
  // assert_eq!(prover.verify(), Ok(()));

  Workbench {
    name: String::from("scalar_mul"),
    params,
    pk,
    vk,
    circuit,
    expected_output,
    rng: rand::rngs::OsRng,
  }
}

pub fn bench_scalar_mul(w: Workbench, crit: &mut Criterion) {
//   let Workbench { params, pk, vk, circuit, expected_output, mut rng, name } = w;

//   let prover_str = format!("{}-prover", name);
//   let verifier_str = format!("{}-verifier", name);

//   crit.bench_function(&prover_str, |b| {
//     b.iter(|| {
//       // ref: https://github.com/zcash/halo2/blob/76b3f892a9d598923bbb5a747701fff44ae4c0ea/halo2_gadgets/benches/poseidon.rs#L178
//       // choose a hash function for FS challenges
//       // Why blake2b not poseidon?
//       // > We will replace BLAKE2b with an algebraic hash function in a later version. - Halo 2 authors
//       let mut transcript =
//         halo2_proofs::transcript::Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
//       create_proof(
//         &params,
//         &pk,
//         &[circuit.clone()],
//         &[&[&[expected_output]]],
//         &mut rng,
//         &mut transcript,
//       )
//       .unwrap();
//     })
//   });
}

fn run_bench(c: &mut Criterion) { bench_scalar_mul(workbench(), c); }

criterion_group!(benches, run_bench);
criterion_main!(benches);
