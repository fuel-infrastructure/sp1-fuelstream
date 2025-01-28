#!/usr/bin/env bash
# This script runs a command and then waits for a specified number of minutes before exiting

WAITFORIT_cmdname=${0##*/}

echoerr() { if [[ $WAITFORIT_QUIET -ne 1 ]]; then echo "$@" 1>&2; fi }

usage()
{
    cat << USAGE >&2
Usage:
    $WAITFORIT_cmdname [-q] [-t minutes] -- command args
    -q | --quiet                    Don't output any status messages
    -t MINUTES | --timeout=MINUTES  Time to wait in minutes after executing the command
    -- COMMAND ARGS                 Command with args to execute
USAGE
    exit 1
}

# Trap SIGINT to allow for immediate script termination
trap "echoerr 'Interrupted! Exiting...'; exit 1" SIGINT

# Trap SIGTERM to allow graceful termination
trap "echoerr 'Terminated! Exiting...'; exit 1" SIGTERM

# process arguments
while [[ $# -gt 0 ]]
do
    case "$1" in
        -q | --quiet)
        WAITFORIT_QUIET=1
        shift 1
        ;;
        -t)
        WAITFORIT_TIMEOUT="$2"
        if [[ $WAITFORIT_TIMEOUT == "" ]]; then break; fi
        shift 2
        ;;
        --timeout=*)
        WAITFORIT_TIMEOUT="${1#*=}"
        shift 1
        ;;
        --)
        shift
        WAITFORIT_CLI=("$@")
        break
        ;;
        --help)
        usage
        ;;
        *)
        echoerr "Unknown argument: $1"
        usage
        ;;
    esac
done

if [[ "${WAITFORIT_CLI[@]}" == "" ]]; then
    echoerr "Error: you need to provide a command to execute."
    usage
fi

WAITFORIT_TIMEOUT=${WAITFORIT_TIMEOUT:-0}
WAITFORIT_QUIET=${WAITFORIT_QUIET:-0}

# execute the command
if [[ $WAITFORIT_QUIET -ne 1 ]]; then
    echoerr "Executing command: ${WAITFORIT_CLI[*]}"
fi
"${WAITFORIT_CLI[@]}"
COMMAND_EXIT_STATUS=$?

# Wait for the specified time, respecting SIGINT/SIGTERM
if [[ $WAITFORIT_TIMEOUT -gt 0 ]]; then
    if [[ $WAITFORIT_QUIET -ne 1 ]]; then
        echoerr "Waiting for $WAITFORIT_TIMEOUT minutes..."
    fi

    # Wait in a loop to allow interruption
    SECONDS_LEFT=$(( WAITFORIT_TIMEOUT * 60 ))
    while [[ $SECONDS_LEFT -gt 0 ]]; do
        sleep 1
        ((SECONDS_LEFT--))
    done
else
    if [[ $WAITFORIT_QUIET -ne 1 ]]; then
        echoerr "No wait time specified, exiting immediately after command execution."
    fi
fi

exit $COMMAND_EXIT_STATUS