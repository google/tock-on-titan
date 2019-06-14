// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Test harness components required by rustc.

// -----------------------------------------------------------------------------
// #[test] macro support. As far as I (jrvanwhy) can tell, #[test] wraps each
// test case in an outer function that interacts with the test crate. For the
// following test definition:
//     #[test]
//     fn do_test() -> TestResult {
//     }
// the macro generates a wrapper that resembles:
//     fn do_test_wrapper() -> /*depends on assert_test_result()*/ {
//         assert_test_result(do_test())
//     }
// The wrapper is then referenced by StaticTestFn (note that the return type of
// StaticTestFn must match the return type of assert_test_result()), which is
// passed to test_main_static as part of TestDescAndFn.
// -----------------------------------------------------------------------------

// Converts the output of the test into a result for StaticTestFn. Note that
// this may be generic, as long as the type parameters can be deduced from its
// arguments and return type.
pub fn assert_test_result(result: bool) -> bool { result }

// -----------------------------------------------------------------------------
// Compiler-generated test list types. The compiler generates a [&TestDescAndFn]
// array and passes it to test_main_static.
// -----------------------------------------------------------------------------

// A ShouldPanic enum is required by rustc, but only No seems to be used.
// #[should_panic] probably uses Yes, but isn't supported here (we assume panic
// = "abort").
pub enum ShouldPanic { No }

// Interestingly, these must be tuple structs for tests to compile.
pub struct StaticTestFn(pub fn() -> bool);
pub struct StaticTestName(pub &'static str);

pub struct TestDesc {
    // Indicates a test case should run but not fail the overall test suite.
    // This was introduced in https://github.com/rust-lang/rust/pull/42219. It
    // is not expected to become stable:
    // https://github.com/rust-lang/rust/issues/46488
    pub allow_fail: bool,

    pub ignore: bool,
    pub name: StaticTestName,
    pub should_panic: ShouldPanic,
}

pub struct TestDescAndFn {
    pub desc: TestDesc,
    pub testfn: StaticTestFn,
}

// The test harness's equivalent of main() (it is called by a compiler-generated
// shim).
pub fn test_main_static(tests: &[&TestDescAndFn]) {
    use core::fmt::Write;
    let mut console = libtock::console::Console::new();
    console.write("Starting tests.\n");
    let mut overall_success = true;
    for test_case in tests {
        // Skip ignored test cases.
        let desc = &test_case.desc;
        let name = desc.name.0;
        if desc.ignore {
            let _ = writeln!(console, "Skipping ignored test {}", name);
            continue;
        }

        // Run the test.
        let _ = writeln!(console, "Running test {}", name);
        let succeeded = test_case.testfn.0();
        let _ = writeln!(console, "Finished test {}. Result: {}.", name,
                         if succeeded { "succeeded" } else { "failed" });
        overall_success &= succeeded;
    }
    let _ = writeln!(console, "TEST_FINISHED: {}",
           if overall_success { "SUCCESS" } else { "FAIL" });
}
