mod epoch_data;
pub use epoch_data::EpochData;

mod hash_to_bits;
pub use hash_to_bits::HashToBits;

mod single_update;
pub use single_update::{ConstrainedEpoch, SingleUpdate};

mod pack;
pub use pack::MultipackGadget;

mod epoch_bits;
pub use epoch_bits::EpochBits;

mod epochs;
pub use epochs::{HashToBitsHelper, ValidatorSetUpdate};

// some helpers
use algebra::{bls12_377::Parameters, sw6::Fr, BigInteger, Field, FpParameters, PrimeField};
use r1cs_std::prelude::*;
use r1cs_std::{bls12_377::G2Gadget, fields::fp::FpGadget, Assignment};

type FrGadget = FpGadget<Fr>;
use bls_gadgets::YToBitGadget;

use r1cs_core::{ConstraintSystem, SynthesisError};

#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use crate::epoch_block::EpochBlock;
    use algebra::{bls12_377::G1Projective, Bls12_377};
    use bls_crypto::{
        hash_to_curve::try_and_increment::COMPOSITE_HASH_TO_G1, PublicKey, SIG_DOMAIN,
    };

    pub fn to_option_iter<T: Copy>(it: &[T]) -> Vec<Option<T>> {
        it.iter().map(|t| Some(*t)).collect()
    }

    pub fn hash_epoch(epoch: &EpochData<Bls12_377>) -> G1Projective {
        let mut pubkeys = Vec::new();
        for pk in &epoch.public_keys {
            pubkeys.push(PublicKey::from(pk.unwrap()));
        }

        // Calculate the hash from our to_bytes function
        let epoch_bytes = EpochBlock::new(epoch.index.unwrap(), epoch.maximum_non_signers, pubkeys)
            .encode_to_bytes()
            .unwrap();
        let (hash, _) = COMPOSITE_HASH_TO_G1
            .hash_with_attempt(SIG_DOMAIN, &epoch_bytes, &[])
            .unwrap();

        hash
    }
}

pub(super) fn pack<F: PrimeField, P: FpParameters>(values: &[bool]) -> Vec<F> {
    values
        .chunks(P::CAPACITY as usize)
        .map(|c| {
            let b = F::BigInt::from_bits(c);
            F::from_repr(b)
        })
        .collect::<Vec<_>>()
}

fn to_fr<T: Into<u64>, CS: ConstraintSystem<Fr>>(
    cs: &mut CS,
    num: Option<T>,
) -> Result<FrGadget, SynthesisError> {
    FrGadget::alloc(cs, || Ok(Fr::from(num.get()?.into())))
}

fn fr_to_bits<CS: ConstraintSystem<Fr>>(
    cs: &mut CS,
    input: &FrGadget,
    length: usize,
) -> Result<Vec<Boolean>, SynthesisError> {
    let mut input = input.to_bits(cs.ns(|| "input to bits"))?;
    input.reverse();
    Ok(input[0..length].to_vec())
}

fn g2_to_bits<CS: ConstraintSystem<Fr>>(
    cs: &mut CS,
    input: &G2Gadget,
) -> Result<Vec<Boolean>, SynthesisError> {
    let x_0 = input.x.c0.to_bits(cs.ns(|| "aggregated pub key c0 bits"))?;
    let x_1 = input.x.c1.to_bits(cs.ns(|| "aggregated pub key c1 bits"))?;
    let y_bit =
        YToBitGadget::<Parameters>::y_to_bit_g2(cs.ns(|| "aggregated pub key y bit"), &input)?;
    let mut output = Vec::new();
    output.extend_from_slice(&x_0);
    output.extend_from_slice(&x_1);
    output.push(y_bit);
    Ok(output)
}

fn constrain_bool<F: Field, CS: ConstraintSystem<F>>(
    cs: &mut CS,
    input: &[Option<bool>],
) -> Result<Vec<Boolean>, SynthesisError> {
    input
        .iter()
        .enumerate()
        .map(|(j, b)| Boolean::alloc(cs.ns(|| format!("{}", j)), || b.get()))
        .collect::<Result<Vec<_>, _>>()
}
