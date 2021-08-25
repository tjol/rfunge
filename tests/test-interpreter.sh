#!/bin/bash
#
# test-interpreter.sh - Script to test other interpreters agaainst
#                       the rfunge test suite.
#
# Example usage: tests/test-interpreter.sh ccbi
#
# If your interpreter requires special flags to operate in standards-
# compliant Befunge-98 mode, use them.
# 
# Example: tests/test-interpreter.sh path/to/funge++ -std=be98
#
# The test suite tests some borderline undefined behaviour where
# you may legitimately disagree with the test result. It also tests
# some fingerprints which your interpreter may not implement.
# Edit the list of excluded tests below as required.

EXCLUDE_TESTS=(
# input_reflects.b98 tests that the input commands reflect on EOF,
# which the spec says they must do. However, this test hangs on
# some non-compliant interpreters
#    input_reflects.b98
#
# There is some legitimate disagreement about what limits i should
# push onto the stack, when the input file contains trailing spaces
# and/or newlines. This test is fairly rfunge-specific.
    file_input2.b98
#
# Fingerprints may or may not be implemented
#    BOOL.b98
#    FIXP.b98
#    FPDP.b98
#    FPRT.b98 # also requires FPDP
#    FPSP.b98
#    HRTI.b98
#    JSTR.b98
#    LONG.b98
#    MODU.b98 # requires FIXP
#    MODU2.b98
#    NULL.b98 # also requires BOOL and HRTI
#    REFC.b98
#    ROMA.b98
#    unload.b98 # requires ROMA and FIXP
#
# MODU specifically is under-specified and implementations failing my
# tests may still be considered correct by some (who would be wrong)
#
#    MODU.b98 # reasonable people can disagree
#    MODU2.b98 # reasonable people can disagree
#
# For Funge++:
#    input_reflects.b98 # hangs (ERROR)
#    FIXP.b98 # not implemented
#    FPDP.b98 # not implemented
#    FPSP.b98 # not implemented
#    FPRT.b98 # not implemented
#    JSTR.b98 # not implemented
#    LONG.b98 # not implemented
#    MODU.b98 # test requires FIXP
#    unload.b98 # test requires FIXP
#
# For cfunge:
#    FPRT.b98 # not implemented
#    LONG.b98 # not implemented
)

interpreter=("$@")

# Check that we have an interpreter!
if [[ "$interpreter" = "" ]]
then
    echo Script to run the rfunge test suite on an arbitrary interpreter
    echo
    echo Usage: $0 path/to/interpreter --interpreter-options
    exit 1
fi

# Check for plausibility
interpreter_exec=${interpreter[0]}
if [[ ! -e "$interpreter_exec" ]]
then
    interpreter_exec=$(which "$interpreter_exec" 2>/dev/null)
    if [[ $? -ne 0 ]]
    then
        echo Command not found: ${interpreter[0]}
        exit 2
    fi
fi
if [[ ! -x "$interpreter_exec" ]]
then
    echo Not executable: $interpreter_exec
    exit 2
else
    interpreter[0]=$(realpath "$interpreter_exec")
fi

# We have an interpreter
echo Testing interpreter: "${interpreter[@]}"
echo

# Change to the right directory
if [[ ! -e "$0" ]]
then
    echo >&2 Warning: "$0" does not appear to exist?
    echo >&2 May not be able to find the test cases...
else
    cd "$(dirname "$0")/test_cases"
fi

# Get the test cases
test_cases=()
IFS="
"
for expected_fn in $(ls *.b98.expected)
do
    program_fn="${expected_fn%.expected}"
    if [[ -e "$program_fn" ]]
    then
        excluded=0
        for excl in ${EXCLUDE_TESTS[@]}
        do
            if [[ $excl = $program_fn ]]
            then
                excluded=1
            fi
        done
        if [[ $excluded -eq 0 ]]
        then
            test_cases+=("$program_fn")
        else
            echo SKIPPING $program_fn!
        fi
    else
        echo >&2 Warning: Result file $expected_fn does not have a matching program!
    fi
done

for test_case in ${test_cases[@]}
do
    echo -n "TEST ${test_case##*/} ... "
    output_file=$(mktemp)
    ${interpreter[@]} $test_case </dev/null >"$output_file" 2>/dev/null
    returncode=$?
    diff_output=$(diff "$test_case.expected" "$output_file")
    diff_returncode=$?
    rm "$output_file"
    if [[ $diff_returncode -eq 0 ]]
    then
        echo -e "\033[0;32mOK\033[0m"
    else
        if [[ $returncode -eq 0 ]]
        then
            echo -e "\033[0;31mFAILED\033[0m"
        else
            echo -e "\033[0;31mFAILED \033[0;33m(code ${returncode})\033[0m"
            echo FAILED "(code ${returncode})"
        fi
        echo "$diff_output"
        echo
    fi
done

