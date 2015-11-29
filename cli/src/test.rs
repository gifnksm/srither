use std::fs::File;
use std::io::prelude::*;
use rustc_test::{self as test, DynTestFn, DynTestName, ShouldPanic, TestDesc, TestDescAndFn};

use slsr_core::puzzle::Puzzle;
use slsr_solver::{self as solver, Solutions};

use error::AppResult;
use parse_arg::TestConfig;

pub fn run(config: TestConfig) -> AppResult<()> {
    let derive_all = config.derive_all;
    let tests = config.input_files
                      .into_iter()
                      .map(|input| {
                          TestDescAndFn {
                              desc: TestDesc {
                                  name: DynTestName(input.clone()),
                                  ignore: false,
                                  should_panic: ShouldPanic::No,
                              },
                              testfn: DynTestFn(Box::new(move || {
                                  solve(&input, derive_all).unwrap()
                              })),
                          }
                      })
                      .collect();

    test::test_main(&["".to_string()], tests);

    Ok(())
}

fn solve(file: &str, derive_all: bool) -> AppResult<()> {
    let mut buf = String::new();
    let _ = try!(try!(File::open(file)).read_to_string(&mut buf));
    let puzzle = try!(buf.parse::<Puzzle>());

    if derive_all {
        for solution in try!(Solutions::new(&puzzle)) {
            let _ = test::black_box(solution);
        }
    } else {
        let _ = test::black_box(try!(solver::solve(&puzzle)));
    }

    Ok(())
}