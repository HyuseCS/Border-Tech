use srp::client::SrpClient;
use srp::groups::G_2048;
use sha2::Sha256;

fn main() {
    let client = SrpClient::<Sha256>::new(b"user", b"password", &G_2048);
    let a = client.get_a_pub();
    let reply = client.process_reply(b"salt", b"B");
}
