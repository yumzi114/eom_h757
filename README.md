# STM32H757I-EVAL

![STM32H757I-EVAL Board](assets/stm32h757i-eval.jpg)

## Default Setting Template — RTIC Base

A Rust + RTIC dual-core project template for the STM32H757I-EVAL board.

- Dual-core STM32H757
- Cortex-M7 + Cortex-M4 firmware
- Shared SRAM4 communication
- RTIC-based applications
- PAC-level clock configuration
- PAC-level FDCAN1 driver
- Automatic dual-core build and flash
- J-Link GDB Server
- Automatic RTT address detection
- RTT console and file logging

## Clock Configuration

```text
CM7 SYSCLK    400 MHz
HCLK          200 MHz
APB clocks    100 MHz
PLL1Q          80 MHz
FDCAN kernel   80 MHz
```

## Project Structure

```text
.
├── board/
├── common/
├── drivers/
│   └── src/
│       ├── clock.rs
│       ├── fdcan.rs
│       └── lib.rs
├── cm7/
│   └── src/
│       └── main.rs
├── cm4/
│   └── src/
│       └── main.rs
├── assets/
│   └── stm32h757i-eval.jpg
├── logs/
└── run-dual.sh
```

## Core Responsibilities

### Cortex-M7

- Configure the power supply and voltage scaling
- Configure Flash latency
- Configure PLL1 and the system clock
- Run at 400 MHz
- Configure HCLK at 200 MHz
- Configure APB clocks at 100 MHz
- Configure PLL1Q at 80 MHz
- Start the Cortex-M4 core
- Update the CM7 SRAM4 heartbeat
- Read the CM4 SRAM4 status
- Produce RTT debug output

### Cortex-M4

- Update the CM4 SRAM4 heartbeat
- Configure the FDCAN1 kernel clock
- Initialize FDCAN1
- Configure FDCAN message RAM
- Perform internal loopback verification
- Transmit Classic CAN frames in normal mode

## Memory Layout

```text
CM7 Flash     0x0800_0000
CM4 Flash     0x0810_0000
Shared SRAM4  0x3800_0000
```

## Shared SRAM4

The Cortex-M7 and Cortex-M4 cores exchange status information through SRAM4.

```text
0x3800_0000  CM7 magic value
0x3800_0004  CM4 magic value
0x3800_0008  CM7 heartbeat counter
0x3800_000C  CM4 heartbeat counter
```

Magic values:

```text
CM7_MAGIC = 0xC07C_07C7
CM4_MAGIC = 0xC04C_04C4
```

## FDCAN1

FDCAN1 is controlled by the Cortex-M4 core.

The driver is implemented directly on top of the STM32H7 PAC without a high-level HAL abstraction.

### Current Configuration

```text
Mode          Classic CAN
Bitrate       500 kbit/s
Kernel clock   80 MHz
Standard ID   0x123
Payload size   8 bytes
```

Nominal bit timing:

```text
Prescaler  10
TSEG1      13 TQ
TSEG2       2 TQ
SJW         2 TQ
```

Bitrate calculation:

```text
80 MHz / 10 / (1 + 13 + 2)
= 500 kbit/s
```

NBTP register value:

```text
0x02090C01
```

### Message RAM

```text
FDCAN1 register base  0x4000_A000
Message RAM base      0x4000_AC00
```

Current message RAM layout:

```text
Offset 0x00  RX FIFO0 element 0
Offset 0x10  TX buffer 0
```

The CPU writes the CAN identifier, DLC, and payload to message RAM and requests transmission through `TXBAR`.

### GPIO

```text
PA11  FDCAN1_RX
PA12  FDCAN1_TX
AF9
```

The internal loopback path and normal CAN output have both been verified.

A second CAN node or USB-CAN adapter is required to receive an ACK and verify complete bus communication.

## Automated Build, Flash, and RTT

The `run-dual.sh` script handles the complete dual-core workflow.

```text
[0/9] Stop previous J-Link sessions
[1/9] Build CM4
[2/9] Build CM7
[3/9] Verify linked addresses
[4/9] Detect CM7 RTT control block
[5/9] Convert ELF to BIN
[6/9] Generate J-Link flash script
[7/9] Flash CM4 and CM7
[8/9] Start J-Link GDB Server
[9/9] Start CM7 RTT
```

The script:

- Checks required commands and J-Link executables
- Supports `debug` and `release` profiles
- Builds CM4 and CM7 separately
- Verifies each `.vector_table` address
- Extracts `_SEGGER_RTT` automatically from the CM7 ELF
- Converts both ELF files to BIN files
- Prints generated BIN sizes
- Generates a temporary J-Link Commander script
- Erases the target
- Flashes and verifies both firmware images
- Starts J-Link GDB Server
- Waits for the RTT Telnet port
- Sends `RTTCh` and `SetRTTAddr` to J-Link
- Prints RTT output to the terminal
- Stores RTT and GDB Server logs
- Cleans up J-Link, RTT, and temporary script processes on exit

Expected linked addresses:

```text
CM7 .vector_table  0x0800_0000
CM4 .vector_table  0x0810_0000
```

Flash layout:

```text
cm7.bin  -> 0x0800_0000
cm4.bin  -> 0x0810_0000
```

Configured ports:

```text
GDB port         2331
SWO port         2332
Telnet port      2333
RTT Telnet port  19021
```

RTT output is written to:

```text
logs/h757-rtt.log
```

J-Link GDB Server output is written to:

```text
logs/jlink-gdb-server.log
```

The RTT control-block address is not hard-coded. The script extracts `_SEGGER_RTT` from the generated CM7 ELF file and sends the detected address to J-Link.

## Requirements

- Rust toolchain
- `thumbv7em-none-eabihf` Rust target
- ARM GNU Embedded tools
- SEGGER J-Link Software
- STM32H757I-EVAL board

Install the Rust target:

```bash
rustup target add thumbv7em-none-eabihf
```

The script expects SEGGER tools at:

```text
/opt/SEGGER/JLink/JLinkExe
/opt/SEGGER/JLink/JLinkGDBServerCLExe
```

The J-Link serial number, target device, interface, speed, and port settings can be changed near the top of `run-dual.sh`.

## Run

```bash
chmod +x ./run-dual.sh
./run-dual.sh release
```

Debug build:

```bash
./run-dual.sh debug
```

The default profile is `debug`:

```bash
./run-dual.sh
```

Press `Ctrl+C` to stop the RTT client and J-Link GDB Server.

## Documentation

STM32H745/755 and STM32H747/757 documentation:

https://www.st.com/en/microcontrollers-microprocessors/stm32h745-755/documentation.html