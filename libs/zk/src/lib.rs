#![no_std]

use soroban_sdk::{
    Bytes, Env, U256, Vec, contracterror,
    crypto::bls12_381::{Fr, G1_SERIALIZED_SIZE, G1Affine, G2_SERIALIZED_SIZE, G2Affine},
    vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Groth16Error {
    MalformedVerifyingKey = 0,
    MalformedProof = 1,
    MalformedPublicSignals = 2,
    NonCanonicalPublicSignal = 3,
}

const FR_SERIALIZED_SIZE: usize = 32;
const FR_MODULUS_BYTES: [u8; FR_SERIALIZED_SIZE] = [
    0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08, 0x09, 0xa1, 0xd8, 0x05,
    0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01,
];

fn take<const N: usize>(bytes: &Bytes, pos: &mut usize) -> Result<[u8; N], Groth16Error> {
    let end = pos
        .checked_add(N)
        .ok_or(Groth16Error::MalformedPublicSignals)?;
    if end > bytes.len() as usize {
        return Err(Groth16Error::MalformedPublicSignals);
    }

    let mut arr = [0u8; N];
    bytes
        .slice((*pos as u32)..(end as u32))
        .copy_into_slice(&mut arr);
    *pos = end;
    Ok(arr)
}

fn is_canonical_fr_bytes(bytes: &[u8; FR_SERIALIZED_SIZE]) -> bool {
    for i in 0..FR_SERIALIZED_SIZE {
        if bytes[i] < FR_MODULUS_BYTES[i] {
            return true;
        }
        if bytes[i] > FR_MODULUS_BYTES[i] {
            return false;
        }
    }
    false
}

#[derive(Clone)]
pub struct VerificationKey {
    pub alpha: G1Affine,
    pub beta: G2Affine,
    pub gamma: G2Affine,
    pub delta: G2Affine,
    pub ic: Vec<G1Affine>,
}

impl VerificationKey {
    pub fn to_bytes(&self, env: &Env) -> Bytes {
        let mut bytes = Bytes::new(env);
        bytes.append(&Bytes::from_slice(env, &self.alpha.to_bytes().to_array()));
        bytes.append(&Bytes::from_slice(env, &self.beta.to_bytes().to_array()));
        bytes.append(&Bytes::from_slice(env, &self.gamma.to_bytes().to_array()));
        bytes.append(&Bytes::from_slice(env, &self.delta.to_bytes().to_array()));
        // Serialize ic length as u32 (big endian)
        let ic_len = self.ic.len() as u32;
        let ic_len_bytes = ic_len.to_be_bytes();
        bytes.append(&Bytes::from_slice(env, &ic_len_bytes));
        for g1 in self.ic.iter() {
            bytes.append(&Bytes::from_slice(env, &g1.to_bytes().to_array()));
        }
        bytes
    }

    pub fn from_bytes(env: &Env, bytes: &Bytes) -> Result<Self, Groth16Error> {
        let mut pos = 0;

        // Deserialize fields
        let alpha = G1Affine::from_array(
            env,
            &take::<G1_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedVerifyingKey)?,
        );
        let beta = G2Affine::from_array(
            env,
            &take::<G2_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedVerifyingKey)?,
        );
        let gamma = G2Affine::from_array(
            env,
            &take::<G2_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedVerifyingKey)?,
        );
        let delta = G2Affine::from_array(
            env,
            &take::<G2_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedVerifyingKey)?,
        );
        // ic length
        let ic_len_bytes =
            take::<4>(bytes, &mut pos).map_err(|_| Groth16Error::MalformedVerifyingKey)?;
        let ic_len = u32::from_be_bytes(ic_len_bytes) as usize;
        let mut ic = Vec::new(env);
        for _ in 0..ic_len {
            let g1 = G1Affine::from_array(
                env,
                &take::<G1_SERIALIZED_SIZE>(bytes, &mut pos)
                    .map_err(|_| Groth16Error::MalformedVerifyingKey)?,
            );
            ic.push_back(g1);
        }
        Ok(VerificationKey {
            alpha,
            beta,
            gamma,
            delta,
            ic,
        })
    }
}

#[derive(Clone)]
pub struct Proof {
    pub a: G1Affine,
    pub b: G2Affine,
    pub c: G1Affine,
}

impl Proof {
    pub fn to_bytes(&self, env: &Env) -> Bytes {
        let mut bytes = Bytes::new(env);
        bytes.append(&Bytes::from_slice(env, &self.a.to_bytes().to_array()));
        bytes.append(&Bytes::from_slice(env, &self.b.to_bytes().to_array()));
        bytes.append(&Bytes::from_slice(env, &self.c.to_bytes().to_array()));
        bytes
    }

    pub fn from_bytes(env: &Env, bytes: &Bytes) -> Result<Self, Groth16Error> {
        const PROOF_BYTES_LEN: u32 =
            (G1_SERIALIZED_SIZE + G2_SERIALIZED_SIZE + G1_SERIALIZED_SIZE) as u32;
        if bytes.len() != PROOF_BYTES_LEN {
            return Err(Groth16Error::MalformedProof);
        }

        let mut pos = 0;
        let a = G1Affine::from_array(
            env,
            &take::<G1_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedProof)?,
        );
        let b = G2Affine::from_array(
            env,
            &take::<G2_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedProof)?,
        );
        let c = G1Affine::from_array(
            env,
            &take::<G1_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedProof)?,
        );
        Ok(Proof { a, b, c })
    }
}

#[derive(Clone)]
pub struct PublicSignals {
    pub pub_signals: Vec<Fr>,
}

impl PublicSignals {
    pub fn to_bytes(&self, env: &Env) -> Bytes {
        let mut bytes = Bytes::new(env);
        let len = self.pub_signals.len() as u32;
        let len_bytes = len.to_be_bytes();
        bytes.append(&Bytes::from_slice(env, &len_bytes));
        for fr in self.pub_signals.iter() {
            let u256 = fr.to_u256();
            let arr32 = u256.to_be_bytes();
            bytes.append(&arr32);
        }
        bytes
    }

    pub fn from_bytes(env: &Env, bytes: &Bytes) -> Result<Self, Groth16Error> {
        let mut pos = 0;
        // Read length (u32, big-endian)
        let len_bytes =
            take::<4>(bytes, &mut pos).map_err(|_| Groth16Error::MalformedPublicSignals)?;
        let len = u32::from_be_bytes(len_bytes);
        let expected_len = 4u32
            .checked_add(
                len.checked_mul(FR_SERIALIZED_SIZE as u32)
                    .ok_or(Groth16Error::MalformedPublicSignals)?,
            )
            .ok_or(Groth16Error::MalformedPublicSignals)?;
        if expected_len != bytes.len() {
            return Err(Groth16Error::MalformedPublicSignals);
        }

        let mut pub_signals = Vec::new(env);
        for _ in 0..len {
            let arr = take::<FR_SERIALIZED_SIZE>(bytes, &mut pos)
                .map_err(|_| Groth16Error::MalformedPublicSignals)?;
            if !is_canonical_fr_bytes(&arr) {
                return Err(Groth16Error::NonCanonicalPublicSignal);
            }
            let u256 = U256::from_be_bytes(env, &Bytes::from_array(env, &arr));
            let fr = Fr::from_u256(u256);
            pub_signals.push_back(fr);
        }
        Ok(PublicSignals { pub_signals })
    }
}

pub struct Groth16Verifier;

impl Groth16Verifier {
    pub fn verify_proof(
        env: &Env,
        vk: VerificationKey,
        proof: Proof,
        pub_signals: &Vec<Fr>,
    ) -> Result<bool, Groth16Error> {
        let bls = env.crypto().bls12_381();

        // Prepare proof inputs:
        // Compute vk_x = ic[0] + sum(pub_signals[i] * ic[i+1])
        if pub_signals.len() + 1 != vk.ic.len() {
            return Err(Groth16Error::MalformedVerifyingKey);
        }
        let mut vk_x = vk.ic.get(0).unwrap();
        for (s, v) in pub_signals.iter().zip(vk.ic.iter().skip(1)) {
            let prod = bls.g1_mul(&v, &s);
            vk_x = bls.g1_add(&vk_x, &prod);
        }

        // Compute the pairing:
        // e(-A, B) * e(alpha, beta) * e(vk_x, gamma) * e(C, delta) == 1
        let neg_a = -proof.a;
        let vp1 = vec![env, neg_a, vk.alpha, vk_x, proof.c];
        let vp2 = vec![&env, proof.b, vk.beta, vk.gamma, vk.delta];

        Ok(bls.pairing_check(vp1, vp2))
    }
}

#[cfg(test)]
mod test;
