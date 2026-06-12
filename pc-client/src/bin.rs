use srp::{ClientG2048, EphemeralSecret};
use sha2::Sha256;

fn main() {
    let srp_client = ClientG2048::<Sha256>::new();
    let a_sec = EphemeralSecret::generate();
    let a_pub: () = srp_client.compute_public_ephemeral(&a_sec);
}
