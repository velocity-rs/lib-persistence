use log::trace;
use rand::{Rng, distr::Alphanumeric, rng};

pub fn get_random_alphanum(len: usize) -> String {
    let rand_string: String = rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();

    trace!("Random string generated {:?}", rand_string);
    rand_string
}

#[expect(unused)]
pub fn get_random_alphanum_lower(len: usize) -> String {
    let rand_string: String = get_random_alphanum(len).to_lowercase();
    trace!("Random lowercase string generated {:?}", rand_string);
    rand_string
}
