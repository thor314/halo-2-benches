//! A gadget for a multiplication gate
use halo2_proofs::{
  circuit::{AssignedCell, Chip, Layouter, Region, Value},
  pasta::group::ff::Field,
  plonk::{Advice, Column, ConstraintSystem, Error, Fixed, Instance, Selector},
  poly::Rotation,
};

pub use self::chip::ScalarMulChip;

pub trait ScalarMulInstructions<F: Field>: Chip<F> {
  type Num;

  fn load_private(&self, layouter: impl Layouter<F>, a: Value<F>) -> Result<Self::Num, Error>;

  fn load_constant(&self, layouter: impl Layouter<F>, constant: F) -> Result<Self::Num, Error>;

  fn mul(&self, layouter: impl Layouter<F>, a: Self::Num, b: Self::Num)
    -> Result<Self::Num, Error>;

  fn expose_public(
    &self,
    layouter: impl Layouter<F>,
    num: Self::Num,
    row: usize,
  ) -> Result<(), Error>;
}

#[derive(Clone, Debug)]

// Represent a value at a cell
pub struct Number<F: Field>(AssignedCell<F, F>);

impl<F: Field> ScalarMulInstructions<F> for ScalarMulChip<F> {
  type Num = Number<F>;

  // load the private input
  fn load_private(
    &self,
    mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    value: Value<F>,
  ) -> Result<Self::Num, halo2_proofs::plonk::Error> {
    let config = self.config();

    layouter.assign_region(
      || "load private",
      |mut region| {
        region.assign_advice(|| "private input", config.advice[0], 0, || value).map(Number)
      },
    )
  }

  // load the constant
  fn load_constant(&self, mut layouter: impl Layouter<F>, constant: F) -> Result<Self::Num, Error> {
    let config = self.config();

    layouter.assign_region(
      || "load constant",
      |mut region| {
        region
          .assign_advice_from_constant(|| "constant value", config.advice[0], 0, constant)
          .map(Number)
      },
    )
  }

  fn mul(
    &self,
    mut layouter: impl Layouter<F>,
    a: Self::Num,
    b: Self::Num,
  ) -> Result<Self::Num, Error> {
    let config = self.config();

    layouter.assign_region(
      || "mul",
      |mut region: Region<'_, F>| {
        // We only want to use a single multiplication gate in this region,
        // so we enable it at region offset 0; this means it will constrain
        // cells at offsets 0 and 1.
        config.s_mul.enable(&mut region, 0)?;

        // The inputs we've been given could be located anywhere in the circuit,
        // but we can only rely on relative offsets inside this region. So we
        // assign new cells inside the region and constrain them to have the
        // same values as the inputs.
        a.0.copy_advice(|| "lhs", &mut region, config.advice[0], 0)?;
        b.0.copy_advice(|| "rhs", &mut region, config.advice[1], 0)?;

        // Now we can assign the multiplication result, which is to be assigned
        // into the output position.
        let value = a.0.value().copied() * b.0.value();

        // Finally, we do the assignment to the output, returning a
        // variable to be used in another part of the circuit.
        region.assign_advice(|| "lhs * rhs", config.advice[0], 1, || value).map(Number)
      },
    )
  }

  fn expose_public(
    &self,
    mut layouter: impl Layouter<F>,
    num: Self::Num,
    row: usize,
  ) -> Result<(), Error> {
    let config = self.config();

    layouter.constrain_instance(num.0.cell(), config.instance, row)
  }
}

#[derive(Clone, Debug)]
pub struct ScalarMulConfig {
  pub advice:   [Column<Advice>; 2],
  pub instance: Column<Instance>,
  pub s_mul:    Selector,
}

impl ScalarMulConfig {
  pub fn configure<F: Field>(
    meta: &mut ConstraintSystem<F>,
    advice: [Column<Advice>; 2],
    instance: Column<Instance>,
    constant: Column<Fixed>,
  ) -> Self {
    // specify the columns that can be compared used by the constraint system
    meta.enable_equality(instance);
    meta.enable_constant(constant);
    for column in &advice {
      meta.enable_equality(*column);
    }

    // meta selector is used to enable gates
    let s_mul = meta.selector();

    // Define our multiplication gate
    meta.create_gate("mul", |meta| {
      // To implement multiplication, we need three advice cells and a selector
      // cell. We arrange them like so:
      //
      // | a0  | a1  | s_mul |
      // |-----|-----|-------|
      // | lhs | rhs | s_mul |
      // | out |     |       |
      //
      // Gates may refer to any relative offsets we want, but each distinct
      // offset adds a cost to the proof. The most common offsets are 0 (the
      // current row), 1 (the next row), and -1 (the previous row), for which
      // `Rotation` has specific constructors.
      let lhs = meta.query_advice(advice[0], Rotation::cur());
      let rhs = meta.query_advice(advice[1], Rotation::cur());
      let out = meta.query_advice(advice[0], Rotation::next());
      let s_mul = meta.query_selector(s_mul);

      // Finally, we return the polynomial expressions that constrain this gate.
      // For our multiplication gate, we only need a single polynomial constraint.
      //
      // The polynomial expressions returned from `create_gate` will be
      // constrained by the proving system to equal zero. Our expression
      // has the following properties:
      // - When s_mul = 0, any value is allowed in lhs, rhs, and out.
      // - When s_mul != 0, this constrains lhs * rhs = out.
      vec![s_mul * (lhs * rhs - out)]
      // vec![Expression::Constant(F::ZERO)]
    });

    ScalarMulConfig { advice, instance, s_mul }
  }
}

mod chip {
  use std::marker::PhantomData;

  use halo2_proofs::{
    circuit::Chip,
    pasta::group::ff::Field,
    plonk::{Advice, Column, ConstraintSystem, Fixed, Instance, Selector},
    poly::Rotation,
  };

  use super::ScalarMulConfig;

  #[derive(Clone)]
  pub struct ScalarMulChip<F: Field> {
    config:  ScalarMulConfig,
    _marker: PhantomData<F>,
  }
  impl<F: Field> Chip<F> for ScalarMulChip<F> {
    type Config = ScalarMulConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config { &self.config }

    fn loaded(&self) -> &Self::Loaded { &() }
  }

  impl<F: Field> ScalarMulChip<F> {
    // construct a chip from a config, weird naming conventions in this place
    pub fn new(config: <Self as Chip<F>>::Config) -> Self { Self { config, _marker: PhantomData } }
  }
}
