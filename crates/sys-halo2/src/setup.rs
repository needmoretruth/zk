//! Halo 2's setup: public parameters from a hash, then keys for one table. Nothing to trust.

use std::io;
use std::sync::Arc;

use halo2_proofs::pasta::{EqAffine, Fp};
use halo2_proofs::plonk::{Circuit, ConstraintSystem, ProvingKey, keygen_pk, keygen_vk};
use halo2_proofs::poly::commitment::Params;
use zk_circuit::lower::plonkish::Plonkish;
use zk_core::{Control, SystemError};

use crate::circuit::TableCircuit;
use crate::field::PastaFp;

/// Everything proving and verifying one statement needs.
pub(crate) struct Keys {
    /// The domain has `2^k` rows.
    pub(crate) k: u32,
    /// The inner-product commitment's generators, derived by hashing to Vesta.
    pub(crate) params: Params<EqAffine>,
    /// The proving key, which carries the verifying key.
    pub(crate) pk: ProvingKey<EqAffine>,
    /// Size of [`Params::write`]'s output.
    pub(crate) params_bytes: u64,
}

/// Picks the smallest domain, derives the parameters and generates both keys for `table`.
pub(crate) fn generate(
    table: &Arc<Plonkish<PastaFp>>,
    control: &Control,
) -> Result<Keys, SystemError> {
    let k = smallest_k(table.num_rows());
    let params = Params::<EqAffine>::new(k);
    let params_bytes = serialized_size(&params)?;
    control.checkpoint()?;
    let shape = TableCircuit::shape(Arc::clone(table));
    let vk = keygen_vk(&params, &shape).map_err(|e| failed("keygen_vk", e))?;
    control.checkpoint()?;
    let pk = keygen_pk(&params, vk, &shape).map_err(|e| failed("keygen_pk", e))?;
    Ok(Keys { k, params, pk, params_bytes })
}

/// The smallest `k` whose `2^k` rows hold the table plus the rows Halo 2 reserves at the bottom:
/// one per blinding factor and one for the permutation argument's last row.
///
/// The count is asked of Halo 2 itself, from the same column configuration the keys use, because
/// the number of blinding factors depends on how often each advice column is queried.
fn smallest_k(rows: usize) -> u32 {
    let mut meta = ConstraintSystem::<Fp>::default();
    TableCircuit::configure(&mut meta);
    let needed = (rows + meta.blinding_factors() + 1).max(meta.minimum_rows());
    needed.next_power_of_two().trailing_zeros()
}

/// Counts what [`Params::write`] would produce without holding it in memory.
fn serialized_size(params: &Params<EqAffine>) -> Result<u64, SystemError> {
    let mut counter = ByteCounter(0);
    params.write(&mut counter).map_err(|e| SystemError::Failed(format!("Params::write: {e}")))?;
    Ok(counter.0)
}

struct ByteCounter(u64);

impl io::Write for ByteCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 += bytes.len() as u64;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn failed(step: &str, error: halo2_proofs::plonk::Error) -> SystemError {
    SystemError::Failed(format!("{step}: {error}"))
}
