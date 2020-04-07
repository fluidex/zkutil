extern crate clap;
extern crate serde_json;
extern crate bellman_ce;
extern crate zkutil;

use std::fs;
use std::fs::File;
use clap::Clap;
use serde_json::*;
use bellman_ce::pairing::bn256::Bn256;
use zkutil::circom_circuit::{
    prove as prove2,
    verify as verify2,
    create_rng,
    load_params_file,
    proof_to_json_file,
    circuit_from_json_file,
    witness_from_json_file,
    load_proof_json_file,
    load_inputs_json_file,
    create_verifier_sol_file,
    proving_key_json_file,
    verification_key_json_file,
    generate_random_parameters,
};

/// A tool to work with SNARK circuits generated by circom
#[derive(Clap)]
struct Opts {
    #[clap(subcommand)]
    command: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Generate a SNARK proof
    Prove(ProveOpts),
    /// Verify a SNARK proof
    Verify(VerifyOpts),
    /// Generate trusted setup parameters
    Setup(SetupOpts),
    /// Generate verifier smart contract
    GenerateVerifier(GenerateVerifierOpts),
    /// Export proving and verifying keys compatible with snarkjs/websnark
    ExportKeys(ExportKeysOpts),
}

/// A subcommand for generating a SNARK proof
#[derive(Clap)]
struct ProveOpts {
    /// Snark trusted setup parameters file
    #[clap(short = "p", long = "params", default_value = "params.bin")]
    params: String,
    /// Circuit JSON file
    #[clap(short = "c", long = "circuit", default_value = "circuit.json")]
    circuit: String,
    /// Witness JSON file
    #[clap(short = "w", long = "witness", default_value = "witness.json")]
    witness: String,
    /// Output file for proof JSON
    #[clap(short = "r", long = "proof", default_value = "proof.json")]
    proof: String,
    /// Output file for public inputs JSON
    #[clap(short = "i", long = "public", default_value = "public.json")]
    public: String,
}

/// A subcommand for verifying a SNARK proof
#[derive(Clap)]
struct VerifyOpts {
    /// Snark trusted setup parameters file
    #[clap(short = "p", long = "params", default_value = "params.bin")]
    params: String,
    /// Proof JSON file
    #[clap(short = "r", long = "proof", default_value = "proof.json")]
    proof: String,
    /// Public inputs JSON file
    #[clap(short = "i", long = "public", default_value = "public.json")]
    public: String,
}

/// A subcommand for generating a trusted setup parameters
#[derive(Clap)]
struct SetupOpts {
    /// Snark trusted setup parameters file
    #[clap(short = "p", long = "params", default_value = "params.bin")]
    params: String,
    /// Circuit JSON file
    #[clap(short = "c", long = "circuit", default_value = "circuit.json")]
    circuit: String,
}

/// A subcommand for generating a Solidity verifier smart contract
#[derive(Clap)]
struct GenerateVerifierOpts {
    /// Snark trusted setup parameters file
    #[clap(short = "p", long = "params", default_value = "params.bin")]
    params: String,
    /// Output smart contract name
    #[clap(short = "v", long = "verifier", default_value = "verifier.sol")]
    verifier: String,
}

/// A subcommand for exporting proving and verifying keys compatible with snarkjs/websnark
#[derive(Clap)]
struct ExportKeysOpts {
    /// Snark trusted setup parameters file
    #[clap(short = "p", long = "params", default_value = "params.bin")]
    params: String,
    /// Reference proving key generated by snarkjs dummy setup
    #[clap(short = "e", long = "ref", default_value = "proving_key.json")]
    reference_pk: String,
    /// Output proving key file
    #[clap(short = "r", long = "pk", default_value = "pk.json")]
    pk: String,
    /// Output verifying key file
    #[clap(short = "v", long = "vk", default_value = "vk.json")]
    vk: String,
}

fn main() {
    let opts: Opts = Opts::parse();
    match opts.command {
        SubCommand::Prove(o) => {
            prove(o);
        }
        SubCommand::Verify(o) => {
            verify(o);
        }
        SubCommand::Setup(o) => {
            setup(o);
        }
        SubCommand::GenerateVerifier(o) => {
            generate_verifier(o);
        }
        SubCommand::ExportKeys(o) => {
            export_keys(o);
        }
    }
}

fn prove(opts: ProveOpts) {
    let rng = create_rng();
    let params = load_params_file(&opts.params);
    println!("Loading circuit from {}...", opts.circuit);
    let mut circuit = circuit_from_json_file(&opts.circuit);
    circuit.witness = Some(witness_from_json_file::<Bn256>(&opts.witness));
    println!("Proving...");
    let proof = prove2(circuit.clone(), &params, rng).unwrap();
    proof_to_json_file(&proof, &opts.proof).unwrap();
    fs::write(&opts.public, circuit.get_public_inputs_json().as_bytes()).unwrap();
    println!("Saved {} and {}", opts.proof, opts.public);
}

fn verify(opts: VerifyOpts) {
    let params = load_params_file(&opts.params);
    let proof = load_proof_json_file::<Bn256>(&opts.proof);
    let inputs = load_inputs_json_file::<Bn256>(&opts.public);
    let correct = verify2(&params, &proof, &inputs).unwrap();
    if correct {
        println!("Proof is correct");
    } else {
        println!("Proof is invalid!");
        std::process::exit(400);
    }
}

fn setup(opts: SetupOpts) {
    println!("Loading circuit from {}...", opts.circuit);
    let rng = create_rng();
    let circuit = circuit_from_json_file::<Bn256>(&opts.circuit);
    println!("Generating trusted setup parameters...");
    let params = generate_random_parameters(circuit, rng).unwrap();
    println!("Writing to file...");
    let writer = File::create(&opts.params).unwrap();
    params.write(writer).unwrap();
    println!("Saved parameters to {}", opts.params);
}

fn generate_verifier(opts: GenerateVerifierOpts) {
    let params = load_params_file(&opts.params);
    create_verifier_sol_file(&params, &opts.verifier).unwrap();
    println!("Created {}", opts.verifier);
}

fn export_keys(opts: ExportKeysOpts) {
    println!("Exporting {}...", opts.params);
    let params = load_params_file(&opts.params);
    verification_key_json_file(&params, &opts.vk).unwrap();
    if fs::metadata(&opts.reference_pk).is_ok() {
        proving_key_json_file(&params, &opts.pk).unwrap();
        copy_json(opts.reference_pk, &opts.pk);
        println!("Created {} and {}.", opts.pk, opts.vk);
    } else {
        println!("Created {}, proving key export skipped because reference key {} does not exist", opts.vk, opts.reference_pk);
    }
}

fn copy_json(ref_file: String, file: &str) {
    let in_json: Map<String, Value> = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
    let mut reference_json: Map<String, Value> = serde_json::from_str(&fs::read_to_string(ref_file).unwrap()).unwrap();
    for (key, value) in &in_json {
        reference_json[key] = value.clone();
    }
    fs::write(file, serde_json::to_string(&reference_json).unwrap().as_bytes()).unwrap();
}
