pub mod job_runner;

pub use aide_de_camp_core as core;
pub use aide_de_camp_sqlite;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
