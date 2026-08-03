#!/usr/bin/env bash
set -euo pipefail

# ============================================================
# STM32H757 dual-core build / flash / RTT
#
# 처리 순서:
#   1. 기존 J-Link 프로세스 종료
#   2. CM4 빌드
#   3. CM7 빌드
#   4. 링크 주소 검사
#   5. CM7 ELF에서 RTT 주소 자동 추출
#   6. ELF -> BIN 변환
#   7. CM4 / CM7 플래시 및 검증
#   8. J-Link GDB Server 실행 및 연결 유지
#   9. RTT Telnet 포트로 로그 수신
#
# 실행:
#   ./run-dual.sh
#   ./run-dual.sh debug
#   ./run-dual.sh release
# ============================================================

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

TARGET="thumbv7em-none-eabihf"
PROFILE="${1:-debug}"

# ------------------------------------------------------------
# J-Link 설정
# ------------------------------------------------------------

JLINK_DIR="/opt/SEGGER/JLink"

JLINK_EXE="$JLINK_DIR/JLinkExe"
JLINK_GDB_SERVER="$JLINK_DIR/JLinkGDBServerCLExe"

JLINK_SERIAL="53000782"
JLINK_DEVICE="STM32H757XI_M7"
JLINK_INTERFACE="SWD"
JLINK_SPEED="4000"

GDB_PORT="2331"
SWO_PORT="2332"
TELNET_PORT="2333"
RTT_TELNET_PORT="19021"

# ------------------------------------------------------------
# 메모리 주소
# ------------------------------------------------------------

CM7_FLASH_ADDR="0x08000000"
CM4_FLASH_ADDR="0x08100000"

EXPECTED_CM7_VECTOR="08000000"
EXPECTED_CM4_VECTOR="08100000"

RTT_CHANNEL="0"
RTT_ADDRESS=""

# ------------------------------------------------------------
# 프로젝트 출력
# ------------------------------------------------------------

LOG_DIR="$ROOT_DIR/logs"

RTT_LOG="$LOG_DIR/h757-rtt.log"
GDB_SERVER_LOG="$LOG_DIR/jlink-gdb-server.log"

JLINK_FLASH_SCRIPT=""

GDB_SERVER_PID=""
RTT_CLIENT_PID=""

# ------------------------------------------------------------
# 빌드 프로필
# ------------------------------------------------------------

case "$PROFILE" in
    debug)
        CARGO_PROFILE_ARGS=()
        OUT_DIR="$ROOT_DIR/target/$TARGET/debug"
        ;;

    release)
        CARGO_PROFILE_ARGS=(--release)
        OUT_DIR="$ROOT_DIR/target/$TARGET/release"
        ;;

    *)
        echo "Usage: $0 [debug|release]"
        exit 1
        ;;
esac

CM4_ELF="$OUT_DIR/cm4"
CM7_ELF="$OUT_DIR/cm7"

CM4_BIN="$OUT_DIR/cm4.bin"
CM7_BIN="$OUT_DIR/cm7.bin"

# ============================================================
# 종료 처리
# ============================================================

cleanup() {
    local exit_code=$?

    trap - EXIT INT TERM

    echo
    echo "Stopping RTT and J-Link sessions..."

    if [[ -n "${RTT_CLIENT_PID:-}" ]]; then
        kill "$RTT_CLIENT_PID" 2>/dev/null || true
        wait "$RTT_CLIENT_PID" 2>/dev/null || true
    fi

    if [[ -n "${GDB_SERVER_PID:-}" ]]; then
        kill "$GDB_SERVER_PID" 2>/dev/null || true
        wait "$GDB_SERVER_PID" 2>/dev/null || true
    fi

    if [[ -n "${JLINK_FLASH_SCRIPT:-}" ]] &&
       [[ -f "$JLINK_FLASH_SCRIPT" ]]
    then
        rm -f "$JLINK_FLASH_SCRIPT"
    fi

    exit "$exit_code"
}

trap cleanup EXIT INT TERM

# ============================================================
# 공통 함수
# ============================================================

require_command() {
    local command_name="$1"

    if ! command -v "$command_name" >/dev/null 2>&1; then
        echo "ERROR: command not found: $command_name"
        exit 1
    fi
}

require_executable() {
    local file_path="$1"

    if [[ ! -x "$file_path" ]]; then
        echo "ERROR: executable not found: $file_path"
        exit 1
    fi
}

require_file() {
    local file_path="$1"

    if [[ ! -f "$file_path" ]]; then
        echo "ERROR: file not found: $file_path"
        exit 1
    fi
}

stop_old_jlink_sessions() {
    echo
    echo "[0/9] Stop previous J-Link sessions"

    pkill -f JLinkRTTLogger 2>/dev/null || true
    pkill -f JLinkRTTClient 2>/dev/null || true
    pkill -f JLinkGDBServerCLExe 2>/dev/null || true
    pkill -f JLinkGDBServer 2>/dev/null || true
    pkill -f JLinkExe 2>/dev/null || true

    sleep 1
}

wait_for_tcp_port() {
    local host="$1"
    local port="$2"
    local attempts="${3:-100}"

    for ((i = 1; i <= attempts; i++)); do
        if python3 - "$host" "$port" <<'PY' >/dev/null 2>&1
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(0.1)

try:
    sock.connect((host, port))
except OSError:
    sys.exit(1)
finally:
    sock.close()
PY
        then
            return 0
        fi

        if [[ -n "${GDB_SERVER_PID:-}" ]] &&
           ! kill -0 "$GDB_SERVER_PID" 2>/dev/null
        then
            echo "ERROR: J-Link GDB Server terminated"
            cat "$GDB_SERVER_LOG"
            exit 1
        fi

        sleep 0.1
    done

    echo "ERROR: timeout waiting for TCP port $host:$port"
    cat "$GDB_SERVER_LOG"
    exit 1
}

print_header() {
    echo
    echo "============================================================"
    echo " STM32H757 dual-core build / flash / RTT"
    echo "============================================================"
    echo " profile          : $PROFILE"
    echo " target           : $TARGET"
    echo " CM7 flash        : $CM7_FLASH_ADDR"
    echo " CM4 flash        : $CM4_FLASH_ADDR"
    echo " J-Link serial    : $JLINK_SERIAL"
    echo " GDB port         : $GDB_PORT"
    echo " RTT Telnet port  : $RTT_TELNET_PORT"
    echo " RTT log          : $RTT_LOG"
    echo "============================================================"
    echo
}

# ============================================================
# 필수 도구 확인
# ============================================================

require_command cargo
require_command arm-none-eabi-nm
require_command arm-none-eabi-objcopy
require_command arm-none-eabi-objdump
require_command awk
require_command grep
require_command stat
require_command strings
require_command pkill
require_command python3

require_executable "$JLINK_EXE"
require_executable "$JLINK_GDB_SERVER"

mkdir -p "$LOG_DIR"

print_header
stop_old_jlink_sessions

# ============================================================
# 1. CM4 빌드
#
# CM4와 CM7은 MCU feature가 다르므로 반드시 별도의
# cargo 명령으로 빌드해야 한다.
# ============================================================

echo
echo "[1/9] Build CM4"

cargo build \
    --target "$TARGET" \
    -p cm4 \
    "${CARGO_PROFILE_ARGS[@]}"

require_file "$CM4_ELF"

# ============================================================
# 2. CM7 빌드
# ============================================================

echo
echo "[2/9] Build CM7"

cargo build \
    --target "$TARGET" \
    -p cm7 \
    "${CARGO_PROFILE_ARGS[@]}"

require_file "$CM7_ELF"

# ============================================================
# 3. 링크 주소 확인
# ============================================================

echo
echo "[3/9] Verify linked addresses"

CM4_VECTOR_ADDR="$(
    arm-none-eabi-objdump -h "$CM4_ELF" |
        awk '$2 == ".vector_table" {
            print toupper($4)
            exit
        }'
)"

CM7_VECTOR_ADDR="$(
    arm-none-eabi-objdump -h "$CM7_ELF" |
        awk '$2 == ".vector_table" {
            print toupper($4)
            exit
        }'
)"

if [[ -z "$CM4_VECTOR_ADDR" ]]; then
    echo "ERROR: CM4 .vector_table section not found"
    exit 1
fi

if [[ -z "$CM7_VECTOR_ADDR" ]]; then
    echo "ERROR: CM7 .vector_table section not found"
    exit 1
fi

echo "CM4 .vector_table: 0x$CM4_VECTOR_ADDR"
echo "CM7 .vector_table: 0x$CM7_VECTOR_ADDR"

if [[ "$CM4_VECTOR_ADDR" != "$EXPECTED_CM4_VECTOR" ]]; then
    echo "ERROR: CM4 must be linked at 0x$EXPECTED_CM4_VECTOR"
    exit 1
fi

if [[ "$CM7_VECTOR_ADDR" != "$EXPECTED_CM7_VECTOR" ]]; then
    echo "ERROR: CM7 must be linked at 0x$EXPECTED_CM7_VECTOR"
    exit 1
fi

# ============================================================
# 4. RTT 주소 자동 추출
# ============================================================

echo
echo "[4/9] Detect CM7 RTT control block"

RTT_ADDRESS="$(
    arm-none-eabi-nm -an "$CM7_ELF" |
        awk '$3 == "_SEGGER_RTT" {
            print toupper($1)
            exit
        }'
)"

if [[ -z "$RTT_ADDRESS" ]]; then
    echo "ERROR: _SEGGER_RTT symbol not found in CM7 ELF"
    exit 1
fi

RTT_ADDRESS="${RTT_ADDRESS#0x}"
RTT_ADDRESS="${RTT_ADDRESS#0X}"

echo "CM7 RTT address: 0x$RTT_ADDRESS"

# ============================================================
# 5. ELF -> BIN 변환
# ============================================================

echo
echo "[5/9] Convert ELF to BIN"

rm -f "$CM4_BIN" "$CM7_BIN"

arm-none-eabi-objcopy \
    -O binary \
    "$CM4_ELF" \
    "$CM4_BIN"

arm-none-eabi-objcopy \
    -O binary \
    "$CM7_ELF" \
    "$CM7_BIN"

require_file "$CM4_BIN"
require_file "$CM7_BIN"

CM4_BIN_SIZE="$(stat -c '%s' "$CM4_BIN")"
CM7_BIN_SIZE="$(stat -c '%s' "$CM7_BIN")"

echo "CM4 BIN      : $CM4_BIN"
echo "CM4 BIN size : $CM4_BIN_SIZE bytes"

echo
echo "CM7 BIN      : $CM7_BIN"
echo "CM7 BIN size : $CM7_BIN_SIZE bytes"

echo
echo "CM7 embedded messages:"

strings "$CM7_ELF" |
    grep -E \
        'STM32H757|RCC_GCR|Waiting for CM4|waiting for CM4|CM4 BOOT' ||
    true

# ============================================================
# 6. J-Link 플래시 스크립트 생성
# ============================================================

echo
echo "[6/9] Generate J-Link flash script"

JLINK_FLASH_SCRIPT="$(mktemp)"

cat >"$JLINK_FLASH_SCRIPT" <<EOF
device $JLINK_DEVICE
si $JLINK_INTERFACE
speed $JLINK_SPEED
connect

r
h
erase

loadbin $CM4_BIN, $CM4_FLASH_ADDR
verifybin $CM4_BIN, $CM4_FLASH_ADDR

loadbin $CM7_BIN, $CM7_FLASH_ADDR
verifybin $CM7_BIN, $CM7_FLASH_ADDR

r
g
sleep 1500
exit
EOF

echo "J-Link script: $JLINK_FLASH_SCRIPT"

# ============================================================
# 7. CM4 / CM7 플래시
# ============================================================

echo
echo "[7/9] Flash CM4 and CM7"

"$JLINK_EXE" \
    -USB "$JLINK_SERIAL" \
    -CommanderScript "$JLINK_FLASH_SCRIPT"

# ============================================================
# 8. GDB Server 실행
#
# -nohalt:
#   서버 접속 시 정상 실행 중인 CM7을 멈추지 않는다.
#
# -RTTTelnetPort:
#   RTT 데이터를 받을 로컬 TCP 포트.
# ============================================================

echo
echo "[8/9] Start J-Link GDB Server"

: >"$GDB_SERVER_LOG"

"$JLINK_GDB_SERVER" \
    -device "$JLINK_DEVICE" \
    -if "$JLINK_INTERFACE" \
    -speed "$JLINK_SPEED" \
    -endian little \
    -USB "$JLINK_SERIAL" \
    -port "$GDB_PORT" \
    -swoport "$SWO_PORT" \
    -telnetport "$TELNET_PORT" \
    -RTTTelnetPort "$RTT_TELNET_PORT" \
    -nohalt \
    -noir \
    -nogui \
    -strict \
    >"$GDB_SERVER_LOG" 2>&1 &

GDB_SERVER_PID=$!

echo "GDB Server PID : $GDB_SERVER_PID"
echo "GDB Server log : $GDB_SERVER_LOG"

wait_for_tcp_port "127.0.0.1" "$RTT_TELNET_PORT" 100

echo "RTT Telnet port ready: 127.0.0.1:$RTT_TELNET_PORT"

# 포트 확인용 접속이 끝난 직후 서버가 안정화될 시간을 준다.
sleep 0.3

# ============================================================
# 9. RTT Telnet 로그
#
# RTT Telnet 연결 직후 100 ms 이내에 SEGGER 설정 문자열을
# 전달한다.
#
# SetRTTAddr:
#   ELF에서 검출한 실제 _SEGGER_RTT 주소 지정
#
# RTTCh:
#   RTT Up Channel 0 선택
# ============================================================

echo
echo "[9/9] Start CM7 RTT"
echo "RTT address : 0x$RTT_ADDRESS"
echo "RTT channel : $RTT_CHANNEL"
echo "RTT endpoint: 127.0.0.1:$RTT_TELNET_PORT"
echo "Log file    : $RTT_LOG"
echo "Press Ctrl+C to stop."
echo

: >"$RTT_LOG"

python3 -u - \
    "$RTT_TELNET_PORT" \
    "$RTT_ADDRESS" \
    "$RTT_CHANNEL" \
    "$RTT_LOG" <<'PY' &

import socket
import sys
import time
from pathlib import Path

HOST = "127.0.0.1"
PORT = int(sys.argv[1])
RTT_ADDRESS = sys.argv[2]
RTT_CHANNEL = int(sys.argv[3])
LOG_PATH = Path(sys.argv[4])

config = (
    "$$SEGGER_TELNET_ConfigStr="
    f"RTTCh;{RTT_CHANNEL};"
    f"SetRTTAddr;0x{RTT_ADDRESS};"
    "$$"
).encode("ascii")

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.settimeout(5.0)

try:
    sock.connect((HOST, PORT))

    # SEGGER 설정 문자열은 연결 직후 바로 전송한다.
    sock.sendall(config)

    sock.settimeout(None)

    with LOG_PATH.open("ab", buffering=0) as log_file:
        while True:
            data = sock.recv(4096)

            if not data:
                raise ConnectionError("RTT Telnet connection closed")

            log_file.write(data)

            sys.stdout.buffer.write(data)
            sys.stdout.buffer.flush()

except KeyboardInterrupt:
    pass

except Exception as exc:
    print(f"\nRTT client error: {exc}", file=sys.stderr)
    sys.exit(1)

finally:
    try:
        sock.close()
    except Exception:
        pass
PY

RTT_CLIENT_PID=$!

wait "$RTT_CLIENT_PID"