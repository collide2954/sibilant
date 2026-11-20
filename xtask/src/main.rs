mod step;
mod task;

use std::env;
use std::process::ExitCode;

use task::Task;

fn main() -> ExitCode {
    let Some(task) = env::args().nth(1).as_deref().and_then(Task::from_name) else {
        eprintln!("{}", Task::usage());
        return ExitCode::FAILURE;
    };
    for step in task.steps() {
        println!("$ {step}");
        match step.run() {
            Ok(status) if status.success() => {}
            Ok(_) => return ExitCode::FAILURE,
            Err(error) => {
                eprintln!("cannot run `{step}`: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
