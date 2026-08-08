# PROJECT_INDEX

Active file tree for **open-servo-core-secure**. Archived files are listed in
[`ARCHIVE_INDEX.md`](ARCHIVE_INDEX.md).

Generated-artifact directories (gerbers, production files, 3D shapes,
footprint libraries, rendered diagrams) are summarised rather than enumerated;
their contents are reproducible from the sources beside them.

## Subsystems at a glance

| Path | What it is |
| ---- | ---------- |
| `docs/` | Normative specifications: wire protocol, transport, control theory, driver pattern, **security architecture** |
| `firmware/lib/` | Chip-agnostic crates — protocol, core, drivers, control table, **osc-security**, host, integration sim |
| `firmware/lib/osc-security/` | **The message plane**: HalfSipHash-2-4 MAC, KDF labels, stream digest, replay window, policy, session state |
| `firmware/lib/drivers/src/bus/` | The osc-native transport: framer, chain, TX engine, clock trim, **security gate** |
| `firmware/lib/drivers/src/se/` | **Secure-element drivers**: ECC204 command layer and SWI-over-UART |
| `firmware/servo-ch32/` | CH32V006 servo runtime: HAL, providers, control kernel |
| `firmware/host-ch32/` | CH32V203 host/adapter runtime |
| `firmware/boards/` | Per-board entry points and memory maps |
| `firmware-old/` | Superseded v1 firmware, retained for reference |
| `hardware/` | KiCad projects: dev board, SG90 swap board, encoder board, motor mount |
| `mechanical/` | Enclosures and fixtures |
| `client/` | Host-side Rust client library and examples |
| `tools/`, `scripts/` | Build and bringup tooling |

## Tree

```text
open-servo-core-secure/
├── .github/
│   ├── workflows/
│   │   ├── ci.yml
│   │   └── claude.yml
│   ├── check-iap-header.sh
│   ├── check-soft-arith.sh
│   └── copilot-instructions.md
├── client/
│   ├── examples/
│   │   ├── cycle_soak.rs
│   │   ├── fleet_smoke.rs
│   │   ├── mute_probe.rs
│   │   └── provision.rs
│   ├── src/
│   │   ├── blocking.rs
│   │   ├── client.rs
│   │   ├── common.rs
│   │   ├── cyclic.rs
│   │   ├── error.rs
│   │   ├── fake.rs
│   │   ├── lib.rs
│   │   ├── mgmt.rs
│   │   ├── nusb.rs
│   │   ├── pipe.rs
│   │   ├── session.rs
│   │   └── wire.rs
│   ├── tests/
│   │   └── full_stack.rs
│   ├── .gitignore
│   ├── Cargo.lock
│   ├── Cargo.toml
│   └── rust-toolchain.toml
├── descriptors/
│   └── osc-servo.json
├── docs/
│   ├── control/  *(11 generated files, not enumerated)*
│   ├── sg90-clones/  *(2 generated files, not enumerated)*
│   ├── 3-Lead-Contact-Package-Usage-DS00004041.pdf
│   ├── ECC204-CryptoAuthentication-Summary-Data-Sheet-DS40002436.pdf
│   ├── Security-Exchange-Process-for-TrustFLEX-and-TrustCUSTOM-Provisioning-DS50004144.pdf
│   ├── SecurityElement.md
│   ├── Trust-Platform-Manifest-File-Full-Format-DS60001759.pdf
│   ├── control-theory.md
│   ├── design-history.md
│   ├── driver-pattern.md
│   ├── infineon-optiga-trust-m-datasheet-en.pdf
│   ├── logo-dark.svg
│   ├── logo.svg
│   ├── osc-native-protocol.md
│   ├── osc-servo-transport.md
│   ├── security-architecture.md
│   ├── sg90-motor-encoder-upgrade.md
│   ├── sg90-pot-encoder-upgrade.md
│   └── testing.md
├── firmware/
│   ├── boards/
│   │   ├── osc-adapter-wchlinke/
│   │   │   ├── .cargo/
│   │   │   │   └── config.toml
│   │   │   ├── src/
│   │   │   │   └── main.rs
│   │   │   ├── Cargo.lock
│   │   │   ├── Cargo.toml
│   │   │   ├── README.md
│   │   │   ├── build.rs
│   │   │   ├── memory.x
│   │   │   └── rust-toolchain.toml
│   │   └── osc-dev-v006/
│   │       ├── .cargo/
│   │       │   └── config.toml
│   │       ├── app/
│   │       │   ├── src/
│   │       │   │   └── main.rs
│   │       │   ├── Cargo.toml
│   │       │   ├── build.rs
│   │       │   └── memory.x
│   │       ├── boot/
│   │       │   ├── src/
│   │       │   │   └── main.rs
│   │       │   ├── Cargo.toml
│   │       │   ├── build.rs
│   │       │   └── memory.x
│   │       ├── Cargo.lock
│   │       ├── Cargo.toml
│   │       ├── riscv32ec_zmmul-unknown-none-elf.json
│   │       └── rust-toolchain.toml
│   ├── host-ch32/
│   │   ├── src/
│   │   │   ├── hal/
│   │   │   │   ├── dma.rs
│   │   │   │   ├── flash.rs
│   │   │   │   ├── gpio.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pfic.rs
│   │   │   │   ├── rcc.rs
│   │   │   │   ├── systick.rs
│   │   │   │   ├── tim2cap.rs
│   │   │   │   ├── usart.rs
│   │   │   │   └── usbhs.rs
│   │   │   ├── providers/
│   │   │   │   ├── clocks.rs
│   │   │   │   ├── deadline.rs
│   │   │   │   ├── edges.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── pins.rs
│   │   │   │   ├── ring.rs
│   │   │   │   ├── tx_wire.rs
│   │   │   │   └── usart_baud.rs
│   │   │   ├── runtime/
│   │   │   │   ├── iap.rs
│   │   │   │   ├── init.rs
│   │   │   │   ├── isr.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── registry.rs
│   │   │   │   ├── run.rs
│   │   │   │   ├── trap.rs
│   │   │   │   └── usb.rs
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── lib/
│   │   ├── control-table/
│   │   │   ├── src/
│   │   │   │   ├── map/
│   │   │   │   │   └── tests.rs
│   │   │   │   ├── descriptor.rs
│   │   │   │   ├── lib.rs
│   │   │   │   ├── map.rs
│   │   │   │   ├── region.rs
│   │   │   │   ├── rules.rs
│   │   │   │   └── stage.rs
│   │   │   ├── tests/
│   │   │   │   ├── ui/
│   │   │   │   │   ├── access_reserved.rs
│   │   │   │   │   ├── access_reserved.stderr
│   │   │   │   │   ├── compare_on_ro.rs
│   │   │   │   │   ├── compare_on_ro.stderr
│   │   │   │   │   ├── compare_on_u32.rs
│   │   │   │   │   ├── compare_on_u32.stderr
│   │   │   │   │   ├── dup_same_side_bound.rs
│   │   │   │   │   ├── dup_same_side_bound.stderr
│   │   │   │   │   ├── enum_missing_repr.rs
│   │   │   │   │   ├── enum_missing_repr.stderr
│   │   │   │   │   ├── enum_on_struct.rs
│   │   │   │   │   ├── enum_on_struct.stderr
│   │   │   │   │   ├── enum_repr_i8.rs
│   │   │   │   │   ├── enum_repr_i8.stderr
│   │   │   │   │   ├── enum_repr_u16.rs
│   │   │   │   │   ├── enum_repr_u16.stderr
│   │   │   │   │   ├── enum_variant_with_fields.rs
│   │   │   │   │   ├── enum_variant_with_fields.stderr
│   │   │   │   │   ├── padding_violation.rs
│   │   │   │   │   ├── padding_violation.stderr
│   │   │   │   │   ├── section_size_mismatch.rs
│   │   │   │   │   ├── section_size_mismatch.stderr
│   │   │   │   │   ├── table_base_offset_mismatch.rs
│   │   │   │   │   └── table_base_offset_mismatch.stderr
│   │   │   │   ├── derive_block.rs
│   │   │   │   ├── derive_enum.rs
│   │   │   │   ├── derive_table.rs
│   │   │   │   └── derive_ui.rs
│   │   │   └── Cargo.toml
│   │   ├── control-table-derive/
│   │   │   ├── src/
│   │   │   │   ├── block.rs
│   │   │   │   ├── common.rs
│   │   │   │   ├── enums.rs
│   │   │   │   ├── lib.rs
│   │   │   │   ├── section.rs
│   │   │   │   └── table.rs
│   │   │   └── Cargo.toml
│   │   ├── core/
│   │   │   ├── src/
│   │   │   │   ├── regions/
│   │   │   │   │   ├── calib.rs
│   │   │   │   │   ├── config.rs
│   │   │   │   │   ├── control.rs
│   │   │   │   │   ├── hooks.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── profile.rs
│   │   │   │   │   └── telemetry.rs
│   │   │   │   ├── services/
│   │   │   │   │   ├── bus/
│   │   │   │   │   │   ├── tests/
│   │   │   │   │   │   │   └── mod.rs
│   │   │   │   │   │   ├── dispatch.rs
│   │   │   │   │   │   ├── mod.rs
│   │   │   │   │   │   └── session.rs
│   │   │   │   │   └── mod.rs
│   │   │   │   ├── traits/
│   │   │   │   │   ├── control.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── services.rs
│   │   │   │   ├── debug.rs
│   │   │   │   ├── kernel.rs
│   │   │   │   ├── lib.rs
│   │   │   │   ├── log.rs
│   │   │   │   ├── persist.rs
│   │   │   │   ├── sample.rs
│   │   │   │   └── shared.rs
│   │   │   └── Cargo.toml
│   │   ├── drivers/
│   │   │   ├── src/
│   │   │   │   ├── bus/
│   │   │   │   │   ├── servo_bus/
│   │   │   │   │   │   ├── crc.rs
│   │   │   │   │   │   ├── reply.rs
│   │   │   │   │   │   ├── route.rs
│   │   │   │   │   │   ├── security.rs
│   │   │   │   │   │   └── tests.rs
│   │   │   │   │   ├── tx/
│   │   │   │   │   │   └── tests.rs
│   │   │   │   │   ├── chain.rs
│   │   │   │   │   ├── clock.rs
│   │   │   │   │   ├── decode.rs
│   │   │   │   │   ├── framer.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── servo_bus.rs
│   │   │   │   │   ├── trim.rs
│   │   │   │   │   └── tx.rs
│   │   │   │   ├── mocks/
│   │   │   │   │   ├── bus.rs
│   │   │   │   │   ├── digital_out.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── monotonic.rs
│   │   │   │   ├── se/
│   │   │   │   │   ├── ecc204.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── swi.rs
│   │   │   │   ├── traits/
│   │   │   │   │   ├── bus.rs
│   │   │   │   │   ├── digital_out.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── monotonic.rs
│   │   │   │   ├── bench.rs
│   │   │   │   ├── led.rs
│   │   │   │   ├── lib.rs
│   │   │   │   └── log.rs
│   │   │   └── Cargo.toml
│   │   ├── host/
│   │   │   ├── src/
│   │   │   │   ├── engine/
│   │   │   │   │   ├── framer.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── shape.rs
│   │   │   │   │   ├── tests.rs
│   │   │   │   │   └── wireop.rs
│   │   │   │   ├── link/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── record.rs
│   │   │   │   │   └── server.rs
│   │   │   │   ├── lib.rs
│   │   │   │   ├── testutil.rs
│   │   │   │   └── traits.rs
│   │   │   └── Cargo.toml
│   │   ├── integration/
│   │   │   ├── src/
│   │   │   │   ├── sim/
│   │   │   │   │   ├── core.rs
│   │   │   │   │   ├── cpu.rs
│   │   │   │   │   ├── host.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── providers.rs
│   │   │   │   │   ├── resample.rs
│   │   │   │   │   ├── servo.rs
│   │   │   │   │   ├── store.rs
│   │   │   │   │   ├── support.rs
│   │   │   │   │   └── tests.rs
│   │   │   │   └── lib.rs
│   │   │   ├── tests/
│   │   │   │   ├── support/
│   │   │   │   │   └── mod.rs
│   │   │   │   ├── chains.rs
│   │   │   │   ├── cross_baud.rs
│   │   │   │   ├── host_loop.rs
│   │   │   │   ├── hot_loop.rs
│   │   │   │   ├── persistence.rs
│   │   │   │   ├── protocol.rs
│   │   │   │   ├── resilience.rs
│   │   │   │   ├── timing.rs
│   │   │   │   └── trim.rs
│   │   │   └── Cargo.toml
│   │   ├── log/
│   │   │   ├── src/
│   │   │   │   └── lib.rs
│   │   │   └── Cargo.toml
│   │   ├── osc-protocol/
│   │   │   ├── src/
│   │   │   │   ├── build.rs
│   │   │   │   ├── bytes.rs
│   │   │   │   ├── crc.rs
│   │   │   │   ├── frame.rs
│   │   │   │   ├── group.rs
│   │   │   │   ├── lib.rs
│   │   │   │   ├── models.rs
│   │   │   │   ├── reply.rs
│   │   │   │   ├── table.rs
│   │   │   │   └── wire.rs
│   │   │   └── Cargo.toml
│   │   ├── osc-security/
│   │   │   ├── src/
│   │   │   │   ├── keys.rs
│   │   │   │   ├── lib.rs
│   │   │   │   ├── mac.rs
│   │   │   │   ├── policy.rs
│   │   │   │   ├── replay.rs
│   │   │   │   ├── se.rs
│   │   │   │   ├── session.rs
│   │   │   │   ├── stream.rs
│   │   │   │   └── trailer.rs
│   │   │   └── Cargo.toml
│   │   ├── table-export/
│   │   │   ├── src/
│   │   │   │   └── main.rs
│   │   │   └── Cargo.toml
│   │   ├── units/
│   │   │   ├── src/
│   │   │   │   └── lib.rs
│   │   │   └── Cargo.toml
│   │   ├── Cargo.toml
│   │   └── rust-toolchain.toml
│   ├── servo-ch32/
│   │   ├── src/
│   │   │   ├── cfg/
│   │   │   │   ├── board_wiring.rs
│   │   │   │   ├── chip.rs
│   │   │   │   └── mod.rs
│   │   │   ├── control/  *(6 generated files, not enumerated)*
│   │   │   ├── hal/
│   │   │   │   ├── adc/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v00x.rs
│   │   │   │   ├── afio/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v00x.rs
│   │   │   │   ├── dma/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v1.rs
│   │   │   │   ├── exti/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v00x.rs
│   │   │   │   ├── flash/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v00x.rs
│   │   │   │   ├── gpio/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v0.rs
│   │   │   │   ├── opa/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v00x.rs
│   │   │   │   ├── pfic/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── rv2.rs
│   │   │   │   ├── rcc/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v00x.rs
│   │   │   │   ├── systick/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── rv2.rs
│   │   │   │   ├── timer/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── v3.rs
│   │   │   │   ├── usart/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── usart_common.rs
│   │   │   │   ├── SAFETY.md
│   │   │   │   ├── clocks.rs
│   │   │   │   ├── esig.rs
│   │   │   │   └── mod.rs
│   │   │   ├── providers/
│   │   │   │   ├── config_store.rs
│   │   │   │   ├── crc.rs
│   │   │   │   ├── deadline.rs
│   │   │   │   ├── digital_out.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── monotonic.rs
│   │   │   │   ├── ring.rs
│   │   │   │   ├── tx_wire.rs
│   │   │   │   └── usart_baud.rs
│   │   │   ├── runtime/
│   │   │   │   ├── diag.rs
│   │   │   │   ├── init.rs
│   │   │   │   ├── isr.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── registry.rs
│   │   │   │   ├── run.rs
│   │   │   │   └── statics.rs
│   │   │   ├── lib.rs
│   │   │   ├── log.rs
│   │   │   └── prelude.rs
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   └── osc-config.x
│   └── .gitignore
├── firmware-old/
│   ├── open-servo-control/
│   │   ├── src/
│   │   │   ├── cascade.rs
│   │   │   ├── lib.rs
│   │   │   ├── pid.rs
│   │   │   └── traits.rs
│   │   └── Cargo.toml
│   ├── open-servo-core/
│   │   ├── src/
│   │   │   ├── debug_shell/
│   │   │   │   ├── arg_parser.rs
│   │   │   │   ├── command.rs
│   │   │   │   ├── exec.rs
│   │   │   │   └── mod.rs
│   │   │   ├── safety/
│   │   │   │   ├── compliance_limiter.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── sensor_health.rs
│   │   │   │   ├── thermal_fault.rs
│   │   │   │   └── thresholds.rs
│   │   │   ├── servo_core/
│   │   │   │   ├── features/
│   │   │   │   │   ├── backdrive.rs
│   │   │   │   │   ├── compliance.rs
│   │   │   │   │   ├── limits.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── policy.rs
│   │   │   │   │   ├── safety.rs
│   │   │   │   │   └── thermal.rs
│   │   │   │   ├── config.rs
│   │   │   │   ├── fast.rs
│   │   │   │   ├── internal.rs
│   │   │   │   ├── medium.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── runtime.rs
│   │   │   │   └── slow.rs
│   │   │   ├── accumulator.rs
│   │   │   ├── app.rs
│   │   │   ├── event.rs
│   │   │   ├── fault.rs
│   │   │   ├── inputs.rs
│   │   │   ├── kinematics.rs
│   │   │   ├── lib.rs
│   │   │   ├── outputs.rs
│   │   │   ├── test_harness.rs
│   │   │   ├── test_support.rs
│   │   │   ├── tick.rs
│   │   │   └── timing.rs
│   │   └── Cargo.toml
│   ├── open-servo-firmware-stm32f301/
│   │   ├── .cargo/
│   │   │   └── config.toml
│   │   ├── src/
│   │   │   ├── init/
│   │   │   │   ├── adc.rs
│   │   │   │   ├── gpio.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── nvic.rs
│   │   │   │   ├── rcc.rs
│   │   │   │   ├── tim.rs
│   │   │   │   └── uart.rs
│   │   │   ├── adc_config.rs
│   │   │   ├── board.rs
│   │   │   ├── calibration.rs
│   │   │   ├── config.rs
│   │   │   ├── flash.rs
│   │   │   ├── isr.rs
│   │   │   ├── main.rs
│   │   │   ├── resources.rs
│   │   │   ├── sinks.rs
│   │   │   └── time_driver.rs
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   └── memory.x
│   ├── open-servo-hw/
│   │   ├── src/
│   │   │   ├── motor/
│   │   │   │   ├── bdc.rs
│   │   │   │   ├── bldc.rs
│   │   │   │   └── mod.rs
│   │   │   ├── peripheral/
│   │   │   │   ├── debug_io.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── time.rs
│   │   │   │   └── uart.rs
│   │   │   ├── sensor/
│   │   │   │   ├── current.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── position.rs
│   │   │   │   ├── temperature.rs
│   │   │   │   ├── velocity.rs
│   │   │   │   └── voltage.rs
│   │   │   ├── v2/
│   │   │   │   ├── adc.rs
│   │   │   │   ├── async_primitives.rs
│   │   │   │   ├── board.rs
│   │   │   │   ├── capability.rs
│   │   │   │   ├── io.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── samples.rs
│   │   │   │   └── timebase.rs
│   │   │   ├── config.rs
│   │   │   ├── lib.rs
│   │   │   └── types.rs
│   │   └── Cargo.toml
│   ├── open-servo-hw-utils/
│   │   ├── src/
│   │   │   ├── adc_dma.rs
│   │   │   ├── lib.rs
│   │   │   ├── rtt_async.rs
│   │   │   └── rtt_debug.rs
│   │   └── Cargo.toml
│   ├── open-servo-kernel/
│   │   ├── src/
│   │   │   ├── kernel.rs
│   │   │   ├── lib.rs
│   │   │   ├── state.rs
│   │   │   └── test_support.rs
│   │   └── Cargo.toml
│   ├── open-servo-kernel-api/
│   │   ├── src/
│   │   │   ├── controller.rs
│   │   │   ├── debug_guard.rs
│   │   │   ├── faults.rs
│   │   │   ├── graph.rs
│   │   │   ├── io.rs
│   │   │   ├── kernel.rs
│   │   │   ├── lib.rs
│   │   │   ├── mailbox.rs
│   │   │   ├── mode.rs
│   │   │   ├── ops.rs
│   │   │   ├── rates.rs
│   │   │   ├── reset.rs
│   │   │   ├── role.rs
│   │   │   ├── shadow.rs
│   │   │   ├── telemetry.rs
│   │   │   ├── tick.rs
│   │   │   ├── tick_ctx.rs
│   │   │   ├── ticks.rs
│   │   │   └── wired.rs
│   │   └── Cargo.toml
│   ├── open-servo-macros/
│   │   ├── src/
│   │   │   ├── adc_channels.rs
│   │   │   ├── lib.rs
│   │   │   └── regmap.rs
│   │   └── Cargo.toml
│   ├── open-servo-math/
│   │   ├── src/
│   │   │   ├── compliance_model.rs
│   │   │   ├── filter.rs
│   │   │   ├── gain.rs
│   │   │   ├── lib.rs
│   │   │   ├── ntc.rs
│   │   │   ├── ntc_gen.rs
│   │   │   ├── pid.rs
│   │   │   ├── thermal.rs
│   │   │   └── tick.rs
│   │   └── Cargo.toml
│   ├── open-servo-registry/
│   │   ├── src/
│   │   │   ├── dxl.rs
│   │   │   ├── facade.rs
│   │   │   ├── lib.rs
│   │   │   ├── policy.rs
│   │   │   ├── reg.rs
│   │   │   ├── spec.rs
│   │   │   └── vendor.rs
│   │   └── Cargo.toml
│   ├── open-servo-rpc/
│   │   ├── src/
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── open-servo-runtime/
│   │   ├── src/
│   │   │   ├── comms_service.rs
│   │   │   ├── device.rs
│   │   │   ├── executor.rs
│   │   │   ├── lib.rs
│   │   │   ├── main_loop.rs
│   │   │   ├── runtime.rs
│   │   │   ├── service_primitives.rs
│   │   │   ├── services.rs
│   │   │   ├── shadow.rs
│   │   │   └── uart_bus.rs
│   │   └── Cargo.toml
│   ├── open-servo-runtime-embassy/
│   │   ├── src/
│   │   │   ├── embassy_runtime.rs
│   │   │   ├── lib.rs
│   │   │   ├── macros.rs
│   │   │   ├── primitives.rs
│   │   │   ├── signals.rs
│   │   │   └── tasks.rs
│   │   └── Cargo.toml
│   ├── open-servo-services/
│   │   ├── src/
│   │   │   ├── dxl_req.rs
│   │   │   ├── dxl_rx.rs
│   │   │   ├── lib.rs
│   │   │   ├── persist.rs
│   │   │   ├── rpc.rs
│   │   │   ├── rpc_transport.rs
│   │   │   ├── service_ctx.rs
│   │   │   ├── service_ops.rs
│   │   │   └── task.rs
│   │   └── Cargo.toml
│   ├── open-servo-stm32f301/
│   │   ├── .cargo/
│   │   │   └── config.toml
│   │   ├── src/
│   │   │   ├── init/
│   │   │   │   ├── adc.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── rcc.rs
│   │   │   │   └── tim.rs
│   │   │   ├── adc_config.rs
│   │   │   ├── adc_sample.rs
│   │   │   ├── board.rs
│   │   │   ├── calibration.rs
│   │   │   ├── config.rs
│   │   │   ├── interrupts.rs
│   │   │   ├── main.rs
│   │   │   ├── pwm.rs
│   │   │   ├── sensors.rs
│   │   │   └── system.rs
│   │   ├── Cargo.toml
│   │   ├── Embed.toml
│   │   ├── build.rs
│   │   └── memory.x
│   ├── open-servo-units/
│   │   ├── src/
│   │   │   ├── adc12.rs
│   │   │   ├── centic.rs
│   │   │   ├── centideg.rs
│   │   │   ├── centideg32.rs
│   │   │   ├── deg_per_sec10.rs
│   │   │   ├── effort.rs
│   │   │   ├── encoder_count.rs
│   │   │   ├── helpers.rs
│   │   │   ├── hertz.rs
│   │   │   ├── lib.rs
│   │   │   ├── macros.rs
│   │   │   ├── microsecond.rs
│   │   │   ├── milliamp.rs
│   │   │   ├── millivolt.rs
│   │   │   └── timestamp.rs
│   │   └── Cargo.toml
│   ├── test-uart-ch32v003/
│   │   ├── .cargo/
│   │   │   └── config.toml
│   │   ├── src/
│   │   │   └── main.rs
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── memory.x
│   │   └── riscv32ec-unknown-none-elf.json
│   ├── test-uart-stm32f301/
│   │   ├── .cargo/
│   │   │   └── config.toml
│   │   ├── src/
│   │   │   └── main.rs
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   └── memory.x
│   ├── .gitignore
│   ├── ARCHITECTURE.md
│   ├── COMPLIANCE_TUNING_GUIDE.md
│   ├── Cargo.lock
│   ├── Cargo.toml
│   ├── PID_TUNING_GUIDE.md
│   ├── REGISTER_MAP.md
│   ├── refactor.md
│   └── rust-toolchain.toml
├── hardware/
│   ├── boards/
│   │   ├── encoder-board/
│   │   │   ├── jlcpcb/  *(17 generated files, not enumerated)*
│   │   │   ├── encoder-board.kicad_dru
│   │   │   ├── encoder-board.kicad_pcb
│   │   │   ├── encoder-board.kicad_pro
│   │   │   ├── encoder-board.kicad_sch
│   │   │   ├── fp-lib-table
│   │   │   └── sym-lib-table
│   │   ├── motor-mount/
│   │   │   ├── jlcpcb/  *(16 generated files, not enumerated)*
│   │   │   ├── fp-lib-table
│   │   │   ├── motor-mount.kicad_pcb
│   │   │   ├── motor-mount.kicad_pro
│   │   │   ├── motor-mount.kicad_sch
│   │   │   └── sym-lib-table
│   │   ├── osc-dev-m007/
│   │   │   ├── docs/
│   │   │   │   ├── back.webp
│   │   │   │   └── front.webp
│   │   │   ├── README.md
│   │   │   ├── bridge.kicad_sch
│   │   │   ├── bus.kicad_sch
│   │   │   ├── fp-lib-table
│   │   │   ├── halfbridge.kicad_sch
│   │   │   ├── mcu.kicad_sch
│   │   │   ├── osc-dev-m007.kicad_dru
│   │   │   ├── osc-dev-m007.kicad_pcb
│   │   │   ├── osc-dev-m007.kicad_pro
│   │   │   ├── osc-dev-m007.kicad_sch
│   │   │   ├── power.kicad_sch
│   │   │   ├── sensors.kicad_sch
│   │   │   ├── shunt.kicad_sch
│   │   │   ├── sym-lib-table
│   │   │   └── testpoints.kicad_sch
│   │   ├── osc-dev-v006/
│   │   │   ├── docs/
│   │   │   │   ├── back.webp
│   │   │   │   └── front.webp
│   │   │   ├── CHANGELOG.md
│   │   │   ├── COMMS.kicad_sch
│   │   │   ├── MCU.kicad_sch
│   │   │   ├── MOTOR_DRIVER.kicad_sch
│   │   │   ├── POWER.kicad_sch
│   │   │   ├── README.md
│   │   │   ├── SENSORS.kicad_sch
│   │   │   ├── fp-lib-table
│   │   │   ├── osc-dev-v006.kicad_dru
│   │   │   ├── osc-dev-v006.kicad_pcb
│   │   │   ├── osc-dev-v006.kicad_pro
│   │   │   ├── osc-dev-v006.kicad_sch
│   │   │   └── sym-lib-table
│   │   ├── osc-sg90-v006/
│   │   │   ├── README.md
│   │   │   ├── fp-lib-table
│   │   │   ├── osc-sg90-v006.kicad_dru
│   │   │   ├── osc-sg90-v006.kicad_pcb
│   │   │   ├── osc-sg90-v006.kicad_pro
│   │   │   ├── osc-sg90-v006.kicad_sch
│   │   │   └── sym-lib-table
│   │   └── servo-dev-board-stm32f301/
│   │       ├── jlcpcb/  *(17 generated files, not enumerated)*
│   │       ├── fp-lib-table
│   │       ├── servo-dev-board.kicad_dru
│   │       ├── servo-dev-board.kicad_pcb
│   │       ├── servo-dev-board.kicad_pro
│   │       ├── servo-dev-board.kicad_sch
│   │       └── sym-lib-table
│   ├── shared.3dshapes/  *(70 generated files, not enumerated)*
│   ├── shared.pretty/  *(39 generated files, not enumerated)*
│   ├── templates/
│   │   └── jlc4l_1v6mm/
│   │       ├── meta/
│   │       │   ├── icon.png
│   │       │   ├── info.html
│   │       │   └── meta.json
│   │       ├── fp-lib-table
│   │       ├── jlc4l_1v6mm.kicad_dru
│   │       ├── jlc4l_1v6mm.kicad_pcb
│   │       ├── jlc4l_1v6mm.kicad_pro
│   │       ├── jlc4l_1v6mm.kicad_sch
│   │       └── sym-lib-table
│   ├── .gitignore
│   ├── README.md
│   └── shared.kicad_sym
├── mechanical/
│   ├── encbench/
│   │   ├── __init__.py
│   │   ├── __main__.py
│   │   └── bench.py
│   ├── sg90/
│   │   ├── __init__.py
│   │   ├── case.py
│   │   ├── fence.py
│   │   ├── gears.py
│   │   ├── measurements.py
│   │   ├── motor.py
│   │   └── pot.py
│   ├── .gitignore
│   ├── README.md
│   ├── render.py
│   └── requirements.txt
├── scripts/
│   └── gears.sh
├── tools/
│   ├── bench/
│   │   ├── src/
│   │   │   ├── bin/
│   │   │   │   ├── tool-baud.rs
│   │   │   │   ├── tool-burst.rs
│   │   │   │   ├── tool-fleet.rs
│   │   │   │   ├── tool-flood.rs
│   │   │   │   ├── tool-ping.rs
│   │   │   │   ├── tool-profile.rs
│   │   │   │   ├── tool-read.rs
│   │   │   │   ├── tool-seam.rs
│   │   │   │   ├── tool-snoop.rs
│   │   │   │   └── tool-write.rs
│   │   │   ├── cli.rs
│   │   │   ├── discover.rs
│   │   │   ├── edges.rs
│   │   │   ├── lib.rs
│   │   │   ├── osc.rs
│   │   │   ├── run.rs
│   │   │   └── wire.rs
│   │   ├── tests/
│   │   │   └── hardware/
│   │   │       ├── chain.rs
│   │   │       ├── hold_commit.rs
│   │   │       ├── hot_loop.rs
│   │   │       ├── mgmt.rs
│   │   │       ├── mod.rs
│   │   │       ├── ping.rs
│   │   │       ├── profile.rs
│   │   │       ├── read.rs
│   │   │       ├── rescue.rs
│   │   │       ├── silence.rs
│   │   │       ├── support.rs
│   │   │       ├── trim.rs
│   │   │       ├── turnaround.rs
│   │   │       └── write.rs
│   │   ├── .gitignore
│   │   ├── Cargo.lock
│   │   └── Cargo.toml
│   └── osc/
│       ├── src/
│       │   ├── descriptor.rs
│       │   └── main.rs
│       ├── .gitignore
│       ├── Cargo.lock
│       └── Cargo.toml
├── .editorconfig
├── .gitattributes
├── .gitignore
├── AGENTS.md
├── CLAUDE.md
├── LICENSE-APACHE
├── LICENSE-HARDWARE
├── LICENSE-MIT
├── README.md
├── REFERENCES.md
└── TODO.md
```
