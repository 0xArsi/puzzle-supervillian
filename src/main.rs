use ark_bls12_381::{g2::Config, Bls12_381, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{
    hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve}, pairing::Pairing, AffineRepr, CurveGroup, Group
};

use ark_ff::{field_hashers::DefaultFieldHasher, BigInt, Field};

use ark_serialize::{CanonicalDeserialize, Read};

use prompt::{puzzle, welcome};

use sha2::Sha256;
use std::{fs::File, ops::Sub};
use std::io::Cursor;
use std::ops::{Mul, Neg};

use ark_std::{rand::SeedableRng, UniformRand, Zero};

fn derive_point_for_pok(i: usize) -> G2Affine {
    let rng = &mut ark_std::rand::rngs::StdRng::seed_from_u64(20399u64);
    G2Affine::rand(rng).mul(Fr::from(i as u64 + 1)).into()
}

#[allow(dead_code)]
fn pok_prove(sk: Fr, i: usize) -> G2Affine {
    derive_point_for_pok(i).mul(sk).into()
}

// pok_verify(new_key, new_key_index, new_proof);
//@note the point in G2 used for the proof/pairing is not G2Affine::generator()
//@note e(pk, (i+1)H) = e(G, sk * (i+1) * H)
fn pok_verify(pk: G1Affine, i: usize, proof: G2Affine) {
    assert!(Bls12_381::multi_pairing(
        &[pk, G1Affine::generator()],
        &[derive_point_for_pok(i).neg(), proof]
    )
    .is_zero());
}

fn hasher() -> MapToCurveBasedHasher<G2Projective, DefaultFieldHasher<Sha256, 128>, WBMap<Config>> {
    let wb_to_curve_hasher =
        MapToCurveBasedHasher::<G2Projective, DefaultFieldHasher<Sha256, 128>, WBMap<Config>>::new(
            &[1, 3, 3, 7],
        )
        .unwrap();
    wb_to_curve_hasher
}

#[allow(dead_code)]
fn bls_sign(sk: Fr, msg: &[u8]) -> G2Affine {
    hasher().hash(msg).unwrap().mul(sk).into_affine()
}

/*
@note
•   checks that e(pk, h(msg)) = e(g1, sk * h(msg)) 
*/
fn bls_verify(pk: G1Affine, sig: G2Affine, msg: &[u8]) {
    assert!(Bls12_381::multi_pairing(
        &[pk, G1Affine::generator()],
        &[hasher().hash(msg).unwrap().neg(), sig]
    )
    .is_zero());
}

fn from_file<T: CanonicalDeserialize>(path: &str) -> T {
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    T::deserialize_uncompressed_unchecked(Cursor::new(&buffer)).unwrap()
}

fn main() {
    welcome();
    puzzle(PUZZLE_DESCRIPTION);

    let public_keys: Vec<(G1Affine, G2Affine)> = from_file("public_keys.bin");

    public_keys
        .iter()
        .enumerate()
        .for_each(|(i, (pk, proof))| pok_verify(*pk, i, *proof));

    let new_key_index = public_keys.len();
    let message = b"0xArsi";

    /* Enter solution here */

   let secret_key = Fr::from(64 as u64);
   let new_key = public_keys
                                   .iter()
                                   .fold(
                                    G1Projective::generator().mul(secret_key), 
                                   |acc, (pk, _)|acc.sub(pk)
                                   ).into_affine();

    //@note each proof p_i = (i * sk_i)H, where H is the random point chosen by derive_point_for_pok
   let new_proof = public_keys
                                   .iter()
                                   .enumerate()
                                   .fold(
                                    derive_point_for_pok(new_key_index).mul(secret_key).mul(Fr::from(new_key_index as u64 + 1).inverse().unwrap()),
                                    |acc, (i, (_, proof))|{
                                       let scaled_proof = proof.mul(Fr::from(i as u64 + 1).inverse().unwrap());
                                       acc.sub(scaled_proof)
                                   }
                                   ).mul(Fr::from(new_key_index as u64 + 1)).into_affine();

    
    let aggregate_signature = bls_sign(secret_key, message);
    /* End of solution */
    pok_verify(new_key, new_key_index, new_proof);
    println!("proof of key verification succeeded");
    let aggregate_key = public_keys
       .iter()
       .fold(G1Projective::from(new_key), |acc, (pk, _)| acc + pk)
       .into_affine();
    bls_verify(aggregate_key, aggregate_signature, message);
    println!("aggregate signature verification succeeded");
}

const PUZZLE_DESCRIPTION: &str = r"
Bob has been designing a new optimized signature scheme for his L1 based on BLS signatures. Specifically, he wanted to be able to use the most efficient form of BLS signature aggregation, where you just add the signatures together rather than having to delinearize them. In order to do that, he designed a proof-of-possession scheme based on the B-KEA assumption he found in the the Sapling security analysis paper by Mary Maller [1]. Based the reasoning in the Power of Proofs-of-Possession paper [2], he concluded that his scheme would be secure. After he deployed the protocol, he found it was attacked and there was a malicious block entered the system, fooling all the light nodes...

[1] https://github.com/zcash/sapling-security-analysis/blob/master/MaryMallerUpdated.pdf
[2] https://rist.tech.cornell.edu/papers/pkreg.pdf
";
