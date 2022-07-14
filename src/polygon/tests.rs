use crate::{archive_store, polygon::parser, problem::config};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;

struct FileNamePattern<'a> {
    before: &'a str,
    after: &'a str,
    padding: usize,
}

impl<'a> FileNamePattern<'a> {
    fn from_printf_format(pattern: &'a str) -> Result<Self> {
        if
        // Access to parent directory may lead to sandbox escape
        pattern.contains("/../")
            || pattern.starts_with("../")
            || pattern.ends_with("/..")
            // Absolute path
            || pattern.starts_with("/")
            // OS-dependent path separator
            || pattern.contains('\\')
            // Absolute path (C:/...) or NTFS alternate stream
            || pattern.contains(':')
        {
            bail!(
                "Format string {pattern:?} is invalid: it must be a relative path, and not \
                 contain /../, \\, or :"
            );
        }

        let pat_start_idx = pattern.find('%').with_context(|| {
            format!("Format string {pattern:?} is invalid: it must contain exactly one %d pattern")
        })?;

        let before = &pattern[..pat_start_idx];

        let pat_end_idx = pat_start_idx
            + pattern[pat_start_idx..].find('d').with_context(|| {
                format!(
                    "Format string {pattern:?} is invalid: it must contain exactly one %d pattern"
                )
            })?
            + 1;

        let after = &pattern[pat_end_idx..];

        let pat = &pattern[pat_start_idx..pat_end_idx];
        let mut padding = 0;

        if pat.len() > 2 {
            if pat.bytes().nth(1).unwrap() != b'0' {
                bail!(
                    "Format string {pattern:?} is invalid: the pattern must be either %d or \
                     %0<number>d"
                );
            }
            padding = pat[2..pat.len() - 1].parse().with_context(|| {
                format!(
                    "Format string {pattern:?} is invalid: the pattern must be either %d or \
                     %0<number>d"
                )
            })?;
        }

        if padding >= 128 {
            bail!(
                "Format string {pattern:?} is invalid: for security, the length of padding in the \
                 %0<number>d pattern must not exceed 127"
            )
        }

        Ok(Self {
            before,
            after,
            padding,
        })
    }

    fn format(&self, number: usize) -> String {
        format!(
            "{}{:padding$}{}",
            self.before,
            number,
            self.after,
            padding = self.padding
        )
    }
}

pub fn generate_dependency_graph(judging: &parser::Judging) -> Result<config::DependencyGraph> {
    // Join all the testsets into a single list of tests. This may create duplicates, e.g. if there
    // are pretests and system tests, but that is not a problem because we have deduplication.
    let mut dependents_of: Vec<Vec<u64>> = Vec::new();

    for testset in judging.testset {
        // Sanity checks
        if testset.test_count != testset.tests.len() {
            bail!(
                "Number of tests ({}) does not agree with the reported count ({})",
                testset.tests.len(),
                testset.test_count
            );
        }

        if testset.tests.is_empty() {
            continue;
        }

        // Determine the connection between groups and tests
        let testset_offset = dependents_of.len();
        dependents_of.resize_with(dependents_of.len() + testset.tests.len(), Vec::new);

        let mut tests_by_group = HashMap::new();
        for (test_id, test) in testset.tests.iter().enumerate() {
            tests_by_group
                .entry(test.group)
                .or_insert_with(Vec::new)
                .push(test_id);
        }

        let groups: HashMap<&str, &parser::Group> = testset
            .groups
            .iter()
            .map(|group| (group.name.as_ref(), group))
            .collect();
        if groups.contains_key("") {
            bail!("A group cannot have an empty name");
        }

        for (group_name, tests) in &tests_by_group {
            if tests.is_empty() {
                bail!("Group {group_name} has no tests");
            }
            if !group_name.is_empty() && !groups.contains_key(group_name.as_str()) {
                bail!(
                    "Test #{} is attached to non-existent group {group_name}",
                    tests[0] + 1
                );
            }
        }

        // Create the appropriate dependency connections inside groups
        for (group_name, group) in groups {
            let tests = &tests_by_group[group_name];

            match group.points_policy.as_ref() {
                "complete-group" => {
                    // If all tests pass, the group is considered positive and score is assigned as
                    // usual. If some test fails, no points are assigned for other tests of the
                    // group, including successful tests before the failed test. This means that
                    // every test is a dependency of every other test. We simulate this behavior
                    // with a ring of dependencies

                    // The tests in the group are expected to be judged consecutively, but that may
                    // be suboptimal. If we can reorder tests without changing the external
                    // behavior, allow that.
                    match group.feedback_policy.as_ref() {
                        "none" | "points" => {
                            // Order does not matter, and every test is sort of dependent on every
                            // other test--simulate that with a ring
                            if tests.len() > 1 {
                                for i in 1..tests.len() {
                                    dependents_of[testset_offset + tests[i - 1]]
                                        .push((testset_offset + tests[i]) as u64);
                                }
                                dependents_of[testset_offset + tests.last().unwrap()]
                                    .push((testset_offset + tests[0]) as u64);
                            }
                        }
                        "icpc" => {
                            // The tests are judged from top to bottom--chain of dependencies
                            for i in 1..tests.len() {
                                dependents_of[testset_offset + tests[i - 1]]
                                    .push((testset_offset + tests[i]) as u64);
                            }
                        }
                        "complete" => {
                            // All tests are judged regardless of failures--there are no
                            // dependencies
                        }
                        _ => {
                            bail!("Unknown feedback policy {}", group.feedback_policy);
                        }
                    }
                }
                "each-test" => {
                    // All tests are judged independently--there are no dependencies. Even with icpc
                    // feedback policy, the 'show first failure' behavior should not stop us from
                    // testing every test.
                    match group.feedback_policy.as_ref() {
                        "none" | "points" | "icpc" | "complete" => {}
                        _ => {
                            bail!("Unknown feedback policy {}", group.feedback_policy);
                        }
                    }
                }
                _ => {
                    bail!("Unknown points policy {}", group.points_policy);
                }
            }

            // Handle group dependencies
            if let Some(ref dependencies) = group.dependencies {
                for dependency in dependencies.dependency {
                    let test_dependencies =
                        tests_by_group.get(&dependency.group).with_context(|| {
                            format!(
                                "Group {} depends on non-existent group {}",
                                group_name, dependency.group
                            )
                        })?;
                    // FIXME: Yes, quadratic complexity, screw me
                    for test in test_dependencies {
                        for dependent_test in tests {
                            dependents_of[testset_offset + dependent_test]
                                .push((testset_offset + test) as u64);
                        }
                    }
                }
            }
        }

        // Tests outside groups are considered to use each-test points policy, because there's
        // little sense to use points otherwise, so no dependencies have to be added.
    }

    Ok(config::DependencyGraph {
        dependents_of: dependents_of
            .into_iter()
            .enumerate()
            .map(|(i, vec)| (i as u64, vec))
            .collect(),
    })
}

pub fn add_tests_to_archive(
    polygon_file_reader: &impl Fn(&Path) -> Result<Vec<u8>>,
    archive_store: &archive_store::ArchiveStore,
    problem_xml: &parser::Problem,
    archive: &mut archive_store::Archive,
) -> Result<()> {
    let mut i = 0usize;

    for testset in problem_xml.judging.testset {
        let mut path_patterns = HashMap::new();

        if let Some(ref pattern) = testset.input_path_pattern {
            path_patterns.insert("input", FileNamePattern::from_printf_format(pattern)?);
        }
        if let Some(ref pattern) = testset.answer_path_pattern {
            path_patterns.insert("answer", FileNamePattern::from_printf_format(pattern)?);
        }

        for pattern in testset.path_pattern {
            if pattern.name.contains('/') || pattern.name.contains('\\') {
                bail!(
                    "The name of path pattern {:?} is invalid because it contains a slash",
                    pattern.name
                );
            }
            if path_patterns
                .insert(
                    &pattern.name,
                    FileNamePattern::from_printf_format(&pattern.value)?,
                )
                .is_some()
            {
                bail!("Path pattern for '{}' is specified twice", pattern.name);
            }
        }

        for test_id in 0..testset.tests.len() {
            for (name, pattern) in &path_patterns {
                let path = pattern.format(test_id + 1);

                let data = polygon_file_reader(path.as_ref()).with_context(|| {
                    format!(
                        "Failed to read {name} of test #{} of testset {} from {path:?}",
                        test_id + 1,
                        testset.name
                    )
                })?;

                let handle = archive_store
                    .store_blob(data)
                    .context("Internal storage error")?;

                archive.add_file(format!("tests/{i}.{name}"), handle, false);
            }

            i += 1;
        }
    }
    Ok(())
}
