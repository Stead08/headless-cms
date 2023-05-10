use rand::distributions::Alphanumeric;
use rand::prelude::ThreadRng;
use rand::Rng;

pub fn generate_key(length: usize) -> String {
    let mut rng = rand::thread_rng();
    generate_api_key_with_rng(length, &mut rng)
}

fn generate_api_key_with_rng(length: usize, rng: &mut ThreadRng) -> String {
    rng.sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
