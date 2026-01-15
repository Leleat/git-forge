use std::process::ExitCode;

fn main() -> ExitCode {
    match git_forge::run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:?}");
            ExitCode::FAILURE
        }
    }
}
