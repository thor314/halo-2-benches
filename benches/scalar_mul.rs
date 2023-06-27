#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use halo2_proofs::{
  circuit::{Chip, SimpleFloorPlanner, Value},
  pasta::{group::ff::Field, Fp},
  plonk::{create_proof, Advice, Circuit, Column, ConstraintSystem, Fixed, Instance},
};
use halo_2_benches::gadgets::scalar_mul::*;

// returning a*b
#[derive(Default)]
pub struct ScalarMulCircuit<F: Field> {
  pub a: Value<F>,
  pub b: Value<F>,
}

impl<F: Field> Circuit<F> for ScalarMulCircuit<F> {
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
    let aa = field_chip.mul(layouter.namespace(|| "a * b"), a.clone(), a)?;
    let bb = field_chip.mul(layouter.namespace(|| "b * b"), b.clone(), b)?;
    let c = field_chip.mul(layouter.namespace(|| "aa * bb"), aa, bb)?;

    // and "return" the result as a public input to the circuit
    field_chip.expose_public(layouter.namespace(|| "expose result"), c, 0)
  }
}

pub fn bench_scalar_mul(name: &str, crit: &mut Criterion) {
  // ANCHOR: test-circuit
  // 2^k is the number of rows in our circuit
  let k = 4;

  // Instantiate the circuit with the private inputs.
  let a = Fp::from(2);
  let b = Fp::from(3);
  // just for the sake of demonstration, show we can used fixed columns to load constants
  let constant = Fp::from(1);
  let c = a.square() * b.square() * constant;
  let (a, b) = (Value::known(a), Value::known(b));
  let my_circuit = ScalarMulCircuit { a, b };

  // Arrange the public input. We expose the multiplication result in row 0
  // of the instance column, so we position it there in our public inputs.
  let mut public_inputs = vec![c];

  // // Given the correct public input, our circuit will verify.
  // let prover = MockProver::run(k, &my_circuit, vec![public_inputs.clone()]).unwrap();
  // assert_eq!(prover.verify(), Ok(()));
  let prover_str = format!("{}-prover", name);
  let verifier_str = format!("{}-verifier", name);
  crit.bench_function(&prover_str, |b| {
    b.iter(|| {
      let mut _transcript = ();
      // todo:
      // https://github.com/zcash/halo2/blob/76b3f892a9d598923bbb5a747701fff44ae4c0ea/halo2_gadgets/benches/poseidon.rs#L178
      // create_proof(&params, &pk, &[circuit], &[&[&[output]]], &mut rng, &mut
      // transcript).unwrap();
    })
  });
}

fn run_bench(c: &mut Criterion) { bench_scalar_mul("scalar_mul", c); }

criterion_group!(benches, run_bench);
criterion_main!(benches);

// fn bench_poseidon<S, const WIDTH: usize, const RATE: usize, const L: usize>(
//     name: &str,
//     c: &mut Criterion,
// ) where
//     S: Spec<Fp, WIDTH, RATE> + Copy + Clone,
// {
//     // Initialize the polynomial commitment parameters
//     let params: Params<vesta::Affine> = Params::new(K);

//     let empty_circuit = HashCircuit::<S, WIDTH, RATE, L> {
//         message: Value::unknown(),
//         _spec: PhantomData,
//     };

//     // Initialize the proving key
//     let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
//     let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

//     let prover_name = name.to_string() + "-prover";
//     let verifier_name = name.to_string() + "-verifier";

//     let mut rng = OsRng;
//     let message = (0..L)
//         .map(|_| pallas::Base::random(rng))
//         .collect::<Vec<_>>()
//         .try_into()
//         .unwrap();
//     let output = poseidon::Hash::<_, S, ConstantLength<L>, WIDTH, RATE>::init().hash(message);

//     let circuit = HashCircuit::<S, WIDTH, RATE, L> {
//         message: Value::known(message),
//         _spec: PhantomData,
//     };

//     c.bench_function(&prover_name, |b| {
//         b.iter(|| {
//             let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
//             create_proof(
//                 &params,
//                 &pk,
//                 &[circuit],
//                 &[&[&[output]]],
//                 &mut rng,
//                 &mut transcript,
//             )
//             .expect("proof generation should not fail")
//         })
//     });

//     // Create a proof
//     let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
//     create_proof(
//         &params,
//         &pk,
//         &[circuit],
//         &[&[&[output]]],
//         &mut rng,
//         &mut transcript,
//     )
//     .expect("proof generation should not fail");
//     let proof = transcript.finalize();

//     c.bench_function(&verifier_name, |b| {
//         b.iter(|| {
//             let strategy = SingleVerifier::new(&params);
//             let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
//             assert!(verify_proof(
//                 &params,
//                 pk.get_vk(),
//                 strategy,
//                 &[&[&[output]]],
//                 &mut transcript
//             )
//             .is_ok());
//         });
//     });
// }

// fn criterion_benchmark(c: &mut Criterion) {
//     bench_poseidon::<MySpec<3, 2>, 3, 2, 2>("WIDTH = 3, RATE = 2", c);
//     bench_poseidon::<MySpec<9, 8>, 9, 8, 8>("WIDTH = 9, RATE = 8", c);
//     bench_poseidon::<MySpec<12, 11>, 12, 11, 11>("WIDTH = 12, RATE = 11", c);
// }

// criterion_group!(benches, criterion_benchmark);
// criterion_main!(benches);
