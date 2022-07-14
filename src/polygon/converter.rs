use crate::{
    archive_store,
    polygon::{parser, tests},
    problem::{config, program, strategy, strategy_format},
};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod default_strategies {
    pub const INPUT_OUTPUT: &str = r#"
file %output %stderr %checker_stderr
block invocation
    tactic user
    ro $input as {input}
    rw %output as {output}
    user <{input} >{output} 2>%stderr
block check
    tactic testlib
    checker $input %output $answer 2>%checker_stderr
"#;

    pub const INTERACTIVE: &str = r#"
file %interactor_output %interactor_stderr
pipe %interactor_to_user %user_to_interactor
block invocation
    tactic user
    user <%interactor_to_user >%user_to_interactor
block interaction
    tactic testlib
    rw %interactor_output as output.txt
    interactor $input output.txt $answer <%user_to_interactor >%interactor_to_user 2>%interactor_stderr
block check
    tactic testlib
    checker $input %inteactor_output $answer 2>%checker_stderr
"#;

    pub const RUN_TWICE_NON_INTERACTIVE: &str = r#"
file %run1_output %run1_stderr %run1_checker_stderr %run2_input %run2_output %run2_stderr %run2_checker_stderr
block firstrun
    tactic user
    ro $input as input.txt
    rw %run1_output as output.txt
    user <input.txt >output.txt 2>%run1_stderr
block firstcheck
    tactic testlib
    checker $input %run1_output $answer >%run2_input 2>%run1_checker_stderr
block secondrun
    tactic user
    ro %run2_input as input.txt
    rw %run2_output as output.txt
    user <input.txt >output.txt 2>%run2_stderr
block secondcheck
    tactic testlib
    checker %run2_input %run2_output $answer 2>%run2_checker_stderr
"#;

    pub const RUN_TWICE_ONLY_FIRST_RUN_INTERACTIVE: &str = r#"
file %run1_stderr %run1_interactor_stderr %run2_input %run2_output %run2_stderr %run2_checker_stderr
pipe %run1_interactor_to_user %run1_user_to_interactor
block firstrun
    tactic user
    user <%run1_interactor_to_user >%run1_user_to_interactor 2>%run1_stderr
block firstinteraction
    tactic testlib
    rw %run2_input as output.txt
    interactor $input output.txt $answer <%run1_user_to_interactor >%run1_interactor_to_user 2>%run1_interactor_stderr
block secondrun
    tactic user
    ro %run2_input as input.txt
    rw %run2_output as output.txt
    user <input.txt >output.txt 2>%run2_stderr
block secondcheck
    tactic testlib
    checker %run2_input %run2_output $answer 2>%run2_checker_stderr
"#;

    pub const RUN_TWICE_ALL_RUNS_INTERACTIVE: &str = r#"
file %run1_stderr %run1_interactor_stderr %run2_input %run2_stderr %run2_interactor_output %checker_stderr
pipe %run1_interactor_to_user %run1_user_to_interactor %run2_interactor_to_user %run2_user_to_interactor
block firstrun
    tactic user
    user <%run1_interactor_to_user >%run1_user_to_interactor 2>%run1_stderr
block firstinteraction
    tactic testlib
    rw %run2_input as output.txt
    interactor $input output.txt $answer <%run1_user_to_interactor >%run1_interactor_to_user 2>%run1_interactor_stderr
block secondrun
    tactic user
    user <%run2_interactor_to_user >%run2_user_to_interactor 2>%run2_stderr
block secondinteraction
    tactic testlib
    rw %run2_interactor_output as output.txt
    interactor %run2_input output.txt $answer <%run2_user_to_interactor >%run2_interactor_to_user 2>%run2_interactor_stderr
block check
    tactic testlib
    checker $input %run2_interactor_output $answer 2>%checker_stderr
"#;
}

pub fn create_archive_from_polygon(
    polygon_file_reader: &impl Fn(&Path) -> Result<Vec<u8>>,
    archive_store: &archive_store::ArchiveStore,
) -> Result<()> {
    let problem_xml =
        polygon_file_reader(&Path::new("problem.xml")).context("Failed to read problem.xml")?;
    let problem_xml =
        std::str::from_utf8(&problem_xml).context("problem.xml has invalid encoding")?;
    let problem_xml = parser::parse_problem_xml(&problem_xml)?;

    if problem_xml.assets.checker.type_ != "testlib" {
        bail!(
            "Checker type {:?} is not supported, only 'testlib' is",
            problem_xml.assets.checker.type_
        );
    }

    let parsed_strategy = match problem_xml.assets.strategy {
        Some(ref strategy) => {
            if strategy.source.type_ == "sunwalker.strategy.v1" {
                let strategy = polygon_file_reader(&Path::new(&strategy.source.path))
                    .with_context(|| {
                        format!("Failed to read strategy at {}", strategy.source.path)
                    })?;
                let strategy =
                    std::str::from_utf8(&strategy).context("Strategy has invalid encoding")?;

                strategy_format::parse_sunwalker_strategy(&strategy)?
            } else {
                bail!(
                    "Unknown strategy type {} specified in problem.xml",
                    strategy.source.type_
                );
            }
        }
        None => {
            // Apply default strategy based on heuristics
            let is_run_twice = match problem_xml.judging.run_count {
                None => problem_xml
                    .tags
                    .tag
                    .iter()
                    .any(|tag| tag.value == "run-twice"),
                Some(1) => false,
                Some(2) => true,
                Some(n) => {
                    bail!("{n} runs are not supported");
                }
            };

            let strategy_text = if is_run_twice {
                match problem_xml.assets.interactor {
                    None => default_strategies::RUN_TWICE_NON_INTERACTIVE,
                    Some(ref interactor) => match interactor.runs {
                        None => default_strategies::RUN_TWICE_ONLY_FIRST_RUN_INTERACTIVE,
                        Some(ref runs) => {
                            let runs: Vec<u64> = runs.run.iter().map(|run| run.value).collect();
                            if runs.as_ref() == [1] {
                                default_strategies::RUN_TWICE_ONLY_FIRST_RUN_INTERACTIVE
                            } else if runs.as_ref() == [1, 2] {
                                default_strategies::RUN_TWICE_ALL_RUNS_INTERACTIVE
                            } else {
                                bail!(
                                    "Invalid <runs> contents: must be either [1] or [1, 2], not \
                                     {:?}",
                                    runs
                                );
                            }
                        }
                    },
                }
            } else {
                match problem_xml.assets.interactor {
                    None => default_strategies::INPUT_OUTPUT,
                    Some(_) => default_strategies::INTERACTIVE,
                }
            };

            strategy_format::parse_sunwalker_strategy(
                &strategy_text
                    .replace(
                        "{input}",
                        &strategy_format::encode_string(
                            if problem_xml.judging.input_file.is_empty() {
                                "input.txt"
                            } else {
                                &problem_xml.judging.input_file
                            },
                        ),
                    )
                    .replace(
                        "{output}",
                        &strategy_format::encode_string(
                            if problem_xml.judging.output_file.is_empty() {
                                "output.txt"
                            } else {
                                &problem_xml.judging.output_file
                            },
                        ),
                    ),
            )
            .unwrap()
        }
    };

    let dependency_graph = tests::generate_dependency_graph(&problem_xml.judging)?;

    let mut archive = archive_store::Archive::new();

    let mut programs = HashMap::new();
    add_program(
        &mut archive,
        &mut programs,
        "checker".to_string(),
        &problem_xml.assets.checker.source,
        &problem_xml.assets.checker.binary,
    )?;
    if let Some(ref interactor) = problem_xml.assets.interactor {
        add_program(
            &mut archive,
            &mut programs,
            "interactor".to_string(),
            &interactor.source,
            &interactor.binary,
        )?;
    }

    tests::add_tests_to_archive(
        polygon_file_reader,
        archive_store,
        &problem_xml,
        &mut archive,
    );

    let problem = config::ProblemRevision {
        dependency_graph,
        strategy_factory: strategy::StrategyFactory {
            files: parsed_strategy.files,
            blocks: parsed_strategy.blocks,
            programs,
            root: PathBuf::new(),
        },
    };

    Ok(())
}

fn add_program(
    archive: &mut archive_store::Archive,
    programs: &mut HashMap<String, program::CachedProgram>,
    name: String,
    source: &parser::Source,
    binary: &parser::Binary,
) -> Result<()> {
    let binary_name = &binary.path.rsplit_once('/').unzip().1.unwrap_or(&binary.path);

    programs.insert(name, program::CachedProgram {
        pub package: String,
        pub prerequisites: Vec<String>,
        argv: vec![binary_name]
    });

    Ok(())
}
