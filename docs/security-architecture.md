# OSC Security Architecture — ECC204 Secure Element Integration

How an ECC204 CryptoAuthentication device gives the osc-native bus
authenticity and integrity **without** touching the transport's
dispatch-before-verdict spine, its hardware CRC pipeline, its reply-gap
grid, or the control cascade's tick budget.

Companion pillars: `osc-native-protocol.md` (the wire), `osc-servo-transport.md`
(the servo-side transport), `control-theory.md` (the cascade this must not
disturb), `SecurityElement.md` (the originating design note).

Code is authoritative; when this doc and the code disagree, fix the doc.

Standards and datasheet citations use the REF-IDs catalogued in
[`REFERENCES.md`](../REFERENCES.md).

---

## 0. What changed from the design note, and why

`SecurityElement.md` sets the goal exactly right — *"strong integrity and
authenticity protection without clobbering the mcu or wire"* — and its core
instinct is correct and preserved here: **the SE authenticates at boot and
derives an ephemeral session key; per-message work is symmetric and cheap.**

Four specifics in the note do not survive contact with the measured numbers.
They are recorded here because the corrections *are* the design.

### 0.1 The ECC204 cannot be in any per-message path

Measured/published device execution times [REF-SE-002]:

| ECC204 command | execution time |
| -------------- | -------------- |
| `SHA` (covers HMAC-SHA-256) | **80 ms** |
| `SIGN` (ECDSA P-256) | **500 ms** |
| `NONCE`, `COUNTER`, `INFO` | 20 ms |
| `READ` / `WRITE` / `LOCK` | 40 / 80 / 80 ms |
| `GENKEY` | 500 ms |

I/O is slower still. The SWI physical layer is a Microchip-proprietary PWM
single-wire interface specified at **100 kbps** [REF-SE-001 §Features]; the
reference host implementation carries it over a UART at 230 400 baud sending
**one UART character per data bit** (`0x7F` = 1, `0x7D` = 0, LSB first)
[REF-SE-003].

The throughput arithmetic is worth doing carefully, because the obvious
version is wrong. Each data bit costs a whole UART *character*, and a
character is **10** bit-times on the wire (start + 8 data + stop) — not 8. So
one data byte costs 8 × 10 = 80 bit-times:

```text
230 400 baud / 10 bit-times per char / 8 chars per byte = 2 880 byte/s
                                                        ≈ 347 µs per byte
                                                        ≈ 23.0 kbit/s
```

A ~35-byte command plus a ~35-byte response is therefore ~**24 ms** of wire on
top of execution. (Pinned by `transfer_time_us` in
`firmware/lib/drivers/src/se/swi.rs`.)

And one HMAC is not one command. On the ECC204 an HMAC is a **two-command**
`SHA` sequence — `HMAC_START` loads the key, `HMAC_END` supplies the message
and returns the digest, using the device-specific mode values `0x03`/`0x02`
[REF-SE-002]. So:

| operation | commands | execution | SWI I/O | total |
| --------- | -------- | --------- | ------- | ----- |
| `NONCE` (16 random bytes) | 1 | 20 ms | ~24 ms | **~44 ms** |
| **HMAC-SHA-256** | 2 × `SHA` | 160 ms | ~48 ms | **~208 ms** |
| **ECDSA P-256 sign** | 1 × `SIGN` | 500 ms | ~45 ms | **~545 ms** |

One SE *data byte* costs ~11 servo frame turnarounds; one HMAC costs ~6 800.

The servo's measured frame turnaround is **30.4 µs** at 1 M
(`osc-servo-transport.md` §10). The SE is **~3 300× too slow** to participate
in a frame, and ~2 000× too slow to participate in a 20 kHz control tick.

**Consequence:** every SE operation is a cold path, and the note's boot-time
session-key model is the only workable one. This document keeps it and makes
it normative.

### 0.2 An 8-bit truncated HMAC must not replace the CRC

The note proposes *"an 8 bit truncation of its HMAC instead of a CRC."*
Three independent problems:

**(a) It is an integrity regression, not an upgrade.** CRC-16/ARC (§3.2 of
the protocol) gives *hard guarantees*: it detects **all** single-bit, all
double-bit, all odd-count, and all burst errors up to 16 bits. An 8-bit tag
gives a **1-in-256 undetected-error rate against every error pattern**,
including a single flipped bit. The osc-native transport is explicitly
engineered around a noisy wire (§3.4 fault contract, F4 mid-frame framing
errors). Trading a hard guarantee for a 0.39 % blind spot degrades the
property that protects a flight actuator from *wire faults* — by far the more
probable failure — in exchange for adversary resistance. On a control surface
that is the wrong trade in both directions.

**(b) 8 bits is not meaningful adversary resistance.** A 1-in-256 tag is
forged by blind guessing. At 1 Mbaud a minimal frame occupies ~70 µs, so an
attacker with bus access lands a forgery in ~128 attempts ≈ **9 ms**.

**(c) It would cost more CPU than everything else in the firmware combined.**
HMAC-SHA-256 over a short message is two SHA-256 compressions. SHA-256's
compression function is ~64 rounds of ARX; on RV32**E** (16 registers, so the
working set spills) an analytical estimate is ~3 500–4 000 instructions per
block, ≈ 110 µs per block at 48 MHz, ≈ **225 µs per HMAC**. The covered-span
dispatch window at 1 M is **20 µs**. It overruns by ~11×, per frame — while
*deleting* the zero-CPU hardware CRC engine that the entire
dispatch-before-verdict spine is built on (`osc-servo-transport.md` §4.4).

**The fix — and the central idea of this architecture: CRC and MAC do
different jobs, so both stay.**

| | CRC-16/ARC | session MAC |
| --- | --- | --- |
| threat | wire faults, noise, garble | an adversary on the bus |
| guarantee | deterministic, exhaustive up to burst-16 | probabilistic, keyed |
| cost | zero CPU (SPI1 + DMA1 CH3) | software, bounded, budgeted |
| verdict role | gates staged effects (existing) | gates staged effects (added) |

The CRC keeps its hardware pipeline and its position in the frame. The MAC is
**added**, never substituted.

### 0.3 PC1 is the bus on every board — but free pins do exist

The note says the SE consumes *"the last pin"*, and the draft `main.rs.new`
assigns SWI to **PC1**, commenting *"PC1 is used exclusively for SWI, leaving
PC2 free for bus.tx_en"*. **PC1 is not free on any board in this repository**,
and on two of the three it is the bus receive path:

| board | PC1 | PC2 |
| ----- | --- | --- |
| firmware rev-B default config | USART1 **RX** through the 74LVC2G241 (§2) | `TX_EN` |
| `osc-dev-v006` Rev 2A (TSSOP-20) | I²C **SDA**, Qwiic J3, 4K7 pull-up | I²C **SCL** |
| **`osc-sg90-v006`** (UQFN-20, production) | **`/RX`** — bus receive | **free** |

Claiming PC1 kills the bus on the rev-B and SG90 configurations, and collides
with the Qwiic bus on the dev board. The draft's premise is wrong on all
three.

The pin budget is genuinely different per board, so **the SE pin belongs in
`BoardWiring`**, selected per board exactly as `dbg`, `drv_en` and
`bus.tx_en` already are — not hard-coded in a driver.

**`osc-sg90-v006` — the production servo (CH32V006F8U6, UQFN-20).**
Extracted from the board's own netlist, four pins are **unconnected**:

| pad | pin | symbol pin function | status |
| --- | --- | ------------------- | ------ |
| 9 | `PC2` | `T2C2_2` | free |
| 10 | `PC3` | `T1C3 / T2C3_4` | free |
| **19** | **`PD5`** | **`A5 / TX / RX_1`** | **free, UART-capable** |
| **20** | **`PD6`** | **`A6 / RX / TX_1`** | **free, UART-capable** |

So the production board has *four* spare pins, two of them UART-capable, and
the SWI 3-lead contact package (2.5 × 6.5 mm) or 8-pad UDFN (2 × 3 mm) fits
the 10 × 12.5 mm board.

### 0.3.1 Interface trade study — SWI vs I²C on the servo control board

The ECC204 ships in both a 100 kbps PWM single-wire (SWI) and a 400 kHz
Fast-mode I²C variant, fixed at the ordering code [REF-SE-001 §Features,
Table 1]. Evaluated against the *servo control board*
(`osc-sg90-v006`, CH32V006F8U6, UQFN-20, 10 × 12.5 mm, sealed inside an SG90
case beside a 1.8 A H-bridge):

| criterion | SWI | I²C | winner |
| --------- | --- | --- | ------ |
| **Pin-map feasibility** | 1 pin, any free GPIO. `PD5`/`PD6` free **and** UART-capable | needs an SDA/SCL **pair**. `I2C1` maps to `PC1`(SDA)/`PC2`(SCL) — `PC1` is the bus `/RX`. Remap options land on `PD0` (`TX_EN`), `PD1` (`SWDIO`), `PC6`/`PC7` (`IN1`/`IN2`) — all taken | **SWI** (I²C may not route *at all*) |
| **Passives / area** | one pull-up, one net | **two** pull-ups (2 × 0402), two nets | **SWI** |
| **Driver code** | reuses the existing, silicon-proven USART HAL — SWI-over-UART is one byte per bit [REF-SE-003] | **no I²C driver exists** in `servo-ch32/src/hal/`; needs a new peripheral driver plus NACK / arbitration / clock-stretch / bus-recovery handling | **SWI** (57 KB flash budget) |
| **Failure modes** | point-to-point, single master, no arbitration, no stretching | shared-bus wedge (slave holding SDA low) needs a 9-clock recovery — bad in a sealed actuator | **SWI** |
| **ESD** | **> 7 kV** HBM | > 4 kV HBM | **SWI** [REF-SE-001 §3.1] |
| **I/O throughput** | ~23.0 kbit/s ⇒ ~24 ms per transaction | ~400 kbit/s ⇒ ~1.6 ms per transaction | I²C |
| **Boot session cost** (`NONCE` + 3 × HMAC, §3) | ~500 ms exec + ~168 ms I/O ≈ **670 ms** | ~500 ms exec + ~11 ms I/O ≈ **510 ms** | I²C, by ~160 ms |
| **Attestation** (`SIGN`, §5.2) | ~500 ms exec + ~45 ms I/O ≈ **545 ms** | ~500 ms exec + ~3 ms I/O ≈ **503 ms** | I²C, by ~42 ms |

**Verdict: SWI, for the servo control board.**

I²C wins exactly one thing — I/O throughput — and §0.1 already established
that throughput is not the binding constraint: ECC204 **execution time**
dominates every transaction (160 ms per HMAC, 500 ms per signature), and it is
identical on both interfaces. The entire I²C advantage is **~160 ms, once per
boot**, on a path that is already ~670 ms long, runs with the bus quiet, and
never touches the control loop. It shortens a one-time boot delay by a quarter
and buys nothing operational.

Against that, SWI wins on the criteria that actually decide whether the design
is buildable: it is the only interface that is **certain to route** on the
production board, it needs one pull-up instead of two on a 10 × 12.5 mm board,
it reuses a proven UART driver instead of requiring a new I²C peripheral
driver against a 57 KB flash budget, it has no shared-bus wedge failure mode
inside a sealed actuator, and it carries ~75 % more ESD margin on a pin that
sits centimetres from a switching H-bridge.

**This vindicates the original design note.** `SecurityElement.md` chose SWI,
and that choice was right for the servo board; only the specific pin (PC1)
was wrong.

### 0.3.2 Per-board pin assignment

| board | interface | pin | driver |
| ----- | --------- | --- | ------ |
| **`osc-sg90-v006`** (production) | **SWI** | **`PD6`** (or `PD5`) | hardware USART @ 230 400 baud |
| `osc-dev-v006` Rev 2A (bringup) | **SWI** | `PC1` **or** `PC2` via the Qwiic connector J3 | bit-banged in a quiet window (§0.4) |

Using SWI on **both** boards keeps the BOM to **one ECC204 ordering code** and
one provisioning run. The dev board has no free pin, but it does not need one:
the Qwiic connector already carries GND / +3V3 / SDA / SCL, so an SWI breakout
plugs straight in using either data pin as SI/O — provided the Qwiic I²C bus
is not otherwise in use. That is a bringup convenience, not a product path.

> The concrete USART instance and remap index for `PD5`/`PD6` must be read
> from `ch32_metapac::METADATA` — the same source `servo-ch32/build.rs`
> already generates `UsartMapping` from — and **not** assumed. If they resolve
> to USART1 remaps only, they would contend with the bus and the SE falls back
> to bit-banged SWI inside a quiet window (§4.3): acceptable for a cold path,
> but it loses the hardware-timing guarantee. Tracked in `TODO.md` §7.2.
>
> **Procurement action:** the ECC204 ordering code fixes the interface at the
> part number. `ECC204-TFLXAUTH*` and `ECC204-TCS*` codes appear in
> [REF-SE-001] Table 4-1 **without an interface column**, so the SWI suffix
> must be confirmed against the full ordering-code table before the BOM is
> frozen. Tracked in `TODO.md` §7.1.

### 0.4 Bit-banged SWI cannot coexist with the transport

The draft `osc_security.rs` bit-bangs SWI with `nop` delay loops. Beyond the
architecture bug (it calls `cortex_m::asm::nop()` — an **ARM** intrinsic; the
board target is `riscv32ec_zmmul-unknown-none-elf`, a QingKe RV32EC core, so
it cannot compile), the approach is structurally unsound here:

One SWI bit period at 230 400 baud is **4.34 µs**. The transport's USART1 and
SysTick vectors run at PFIC **HIGH** with dispatch bodies of **10–70 µs**
(`osc-servo-transport.md` §2). Any frame arriving mid-transaction stretches a
bit period by an order of magnitude and corrupts the SE exchange. A
`nop`-loop timing base is additionally invalidated by the HSITRIM slewing the
clock-discipline loop performs continuously (§9.3).

**Rule (normative):** SE transactions run only in a **bus-quiet window** —
at boot before the transport starts, or under an explicit host-negotiated
quiet window (§4.3). No SE transaction may be initiated from an ISR, and none
may overlap live bus traffic.

### 0.5 The monotonic counter is a lifecycle counter, not a session counter

The ECC204's monotonic counter saturates at **10 000** [REF-SE-001 §Features;
independently `COUNTER_MAX_VALUE_CA2 = 10000` in REF-SE-002]. It cannot count
messages (10 000 frames ≈ 0.7 s of hot loop) and should not count boots
(10 000 power cycles is inside a UAV's service life).

It is reserved for **lifecycle authorizations**: firmware-update grants and
factory-reset grants — events measured in dozens over a device lifetime. §5.3.

---

## 1. Three planes, three timescales

The design's organizing principle: match each security operation to the
timescale it can afford.

| plane | protects | primitive | who computes | cost | frequency |
| ----- | -------- | --------- | ------------ | ---- | --------- |
| **Identity** | "this is a genuine OSC servo" | ECDSA P-256 + cert chain | ECC204 `SIGN` | ~520 ms | boot, on demand |
| **Session** | key agreement, mutual auth | HMAC-SHA-256 KDF | ECC204 `SHA`/HMAC | ~100 ms | boot, re-key |
| **Message** | every command's authenticity | HalfSipHash-2-4 | MCU software | µs, budgeted | every effect |

The identity and session planes are **cold paths, bus-quiet, torque-off-safe**.
Only the message plane touches the hot loop, and it touches it only where the
transport already had a gate.

---

## 2. The message plane

### 2.1 The insight: authenticate the effect, not the transport

The naive scheme — a MAC on every frame — costs the most exactly where the
budget is tightest. It is also the wrong boundary. The protocol's intended hot
loop (`osc-native-protocol.md` §7) is:

```text
GWRITE(HOLD|NOREPLY) × groups  →  COMMIT (broadcast, silent)  →  GREAD chain
```

`HOLD` writes **stage**; they change nothing. The instant that matters — the
one that moves a control surface — is the **broadcast COMMIT**. So that is
what gets authenticated, and the tag it carries covers *the whole
instruction stream since the last commit*:

- Every servo folds **every instruction frame it observes** — in full, in wire
  order — into a keyed rolling digest. Status frames (`INST` bit 7) are
  excluded; they carry the responder's key, not the host's.
- `COMMIT` carries `(seq, tag)` where the tag is over
  `(epoch, seq, stream_digest)`.
- On a tag mismatch the existing **REVERT** path discards the entire staging
  buffer. Nothing was ever applied.

> **Why the whole frame, not just our own slice.** Folding only the servo's
> own GWRITE slice would be far cheaper, but it cannot work: a **broadcast**
> COMMIT carries exactly one tag, so every servo must arrive at the *same*
> digest. Since the wire is shared, every servo already sees every frame —
> folding the frame in full is what makes the digest fleet-common and lets one
> broadcast tag close the whole cycle. (Per-servo digests would need N tags in
> the COMMIT — 4 bytes each — which costs far more wire than it saves CPU;
> see §2.6.)

The properties this buys:

- An **injected** forged GWRITE enters the digest → COMMIT fails → full revert.
- A **suppressed** legitimate GWRITE leaves the digest short → COMMIT fails →
  full revert. (Jamming becomes fail-safe rather than partial-apply.)
- A **modified** GWRITE slice changes the digest → COMMIT fails → full revert.
- A **reordered** stream changes the digest → COMMIT fails → full revert.
- A **replayed** COMMIT fails the sequence check (§2.5).

**Divergence is fail-safe, and is not a regression.** If one servo drops a
frame on CRC while its peers do not, its digest diverges, its COMMIT fails,
and it reverts and alerts while the fleet commits. Today that same drop means
that servo silently commits a *stale* value; under the digest it holds its
last authenticated state and tells the host. The host's existing
timeout-and-retry contract (§3.4) covers both.

### 2.2 What still needs an inline tag

The atomic-staging trick only covers held writes. Everything else carries the
tag inline:

| class | requirement in a secured session |
| ----- | -------------------------------- |
| `WRITE`/`GWRITE` **with** `HOLD` | fold into digest; no trailer |
| `WRITE`/`GWRITE` **without** `HOLD` | **inline AUTH trailer required** |
| `COMMIT` | **inline AUTH trailer required** (carries the digest verdict) |
| `MGMT` (`SAVE`/`FACTORY`/`REBOOT`/`ASSIGN`) | inline AUTH trailer required |
| `MGMT SAVE`/`FACTORY`, firmware update | AUTH trailer **+ fresh SE grant** (§5.3) |
| `PING`/`READ`/`GREAD` | no table effect → policy-configurable, default off |
| status replies (servo → host) | policy-configurable, default on for telemetry |

The rule that makes this airtight: **an unauthenticated frame may not carry an
effect.** A non-`HOLD` write without a trailer is rejected with the new
`ResultCode::Unauthenticated` rather than applied. The *servo* enforces this
from its policy register — never the host's choice of flag — so an attacker
cannot opt out by clearing a bit.

### 2.3 Wire format — a strictly additive extension

`INST` **bit 1** is reserved-zero in both layouts and explicitly held "for
future extensions" (`osc-native-protocol.md` §3.1, §5; confirmed in
`osc-protocol::wire::Inst`). It becomes **`FLAG_AUTH`**.

```text
BREAK | ID | LEN | INST(AUTH) | payload | SEQ | TAG[0..4] | CRC_lo | CRC_hi
                               \______________________/
                                 5-byte AUTH trailer
                                 (inside the payload region)
```

Everything the transport depends on is untouched:

- The trailer lives **inside the payload region**, so `LEN` accounts for it by
  the existing `len_for(p)` arithmetic and the CRC covers it by the existing
  `ID..payload` definition (§3.2). No new span math.
- Frame anatomy, break law, footprint algebra, ring parity, the hardware CRC
  feed, the chain snoop (§6, which reads framing only), and every host tool
  are unchanged.
- A frame **without** `FLAG_AUTH` is byte-identical to today's protocol.
- Wire cost is **5 bytes**, and only on frames that carry it.

Wire cost in context (1 Mbaud, 10 µs/byte):

| frame | plain | with trailer | delta |
| ----- | ----- | ------------ | ----- |
| `COMMIT` (broadcast) | 6 B / 60 µs | 11 B / 110 µs | +50 µs per cycle |
| 8-target uniform `GWRITE` | 49 B | 49 B (digest, no trailer) | **0** |
| 16 B telemetry status | 21 B | 26 B | +50 µs |
| `PING` | 6 B | 6 B (policy: off) | 0 |

The hot loop pays **one** trailer per cycle, on COMMIT.

### 2.4 The per-frame primitive

Requirements: keyed PRF, ≥32-bit tag, no lookup tables (flash cost and
cache-timing side channels), ARX-only (RV32EC has no rotate instruction, no
divide, and `zmmul` multiply only), and small enough to review.

| candidate | tag | analytical cost, 48 B | verdict |
| --------- | --- | --------------------- | ------- |
| HMAC-SHA-256 (trunc.) | any | ~225 µs | **out** — §0.2(c) |
| SipHash-2-4 (64-bit ARX) | 64 b | ~2× HalfSipHash on RV32 | out — 64-bit words cost 2–4× |
| **HalfSipHash-2-4** | 32 b | ~730 instr ≈ 20–30 µs | **selected** |
| HalfSipHash-1-3 | 32 b | ~390 instr ≈ 11–16 µs | fallback if bench demands |
| NH/UMAC + pad | 32 b | ~40 instr ≈ 1.2 µs | rejected — pad reuse is catastrophic |

**Selected: HalfSipHash-2-4**, 64-bit key, 32-bit tag [REF-CRYPTO-001].

Rationale over the faster NH construction: HalfSipHash is a *stateless PRF*.
NH-style Carter–Wegman MACs need a never-repeating pad per message; a single
pad reuse leaks the hash key and collapses to unlimited forgery. Under a
protocol with resets, rescue breaks, brownouts and re-keys, "the pad never
repeats" is a property that is very easy to break and very hard to test for.
A PRF has no such failure mode, and §2.6 shows the budget does not need the
extra speed badly enough to buy that risk.

HalfSipHash is **not** a NIST-approved primitive. That is a deliberate,
bounded exception, and it is confined: session keys are ephemeral and derived
by a FIPS-approved KDF running inside a certified device (§3), the tag is
short-lived, and forgery is rate-limited by lockout (§2.7). The approved
primitives (HMAC-SHA-256, ECDSA P-256) carry the identity and session planes
where their cost is affordable. See §6 for the compliance treatment.

### 2.5 Replay: epoch and sequence

- **`epoch`** — u16, incremented on every session establishment. The session
  key is bound to it, so a cross-epoch replay fails on the key, not just a
  counter.
- **`seq`** — u32 held internally, **low 8 bits on the wire**. The receiver
  reconstructs the high bits and accepts only `seq > last_seq` within a
  forward window of +127. Strictly increasing, so a replayed COMMIT is
  rejected even within its own epoch.
- Loss tolerance: the protocol drops frames on CRC failure and the host
  retries (§3.4, §5.3 L1). The forward window absorbs those gaps without
  desynchronising; a gap wider than the window forces a re-key, which is a
  cold path and therefore affordable.

The wire-visible `SEQ` byte is what makes the scheme robust to the protocol's
*existing* loss model. A purely implicit counter would desynchronise on the
first dropped frame.

### 2.6 Where the cost lands — and the honest caveat

> **Timing figures in this section are analytical, not measured.** This
> repository's convention is that timing claims are silicon-measured and
> catalogued ([F1]–[F15]). No Rust toolchain was available in the authoring
> environment, so no cycle counts were produced. The instruction counts are
> derived from the primitive's operation structure (HalfSipHash round = 4
> adds + 4 XORs + 6 rotates; a rotate is 3 instructions with no `Zbb`) and
> assume ~1.5 CPI. **They must be confirmed on silicon before this design is
> flown.**
>
> A `bench`-feature probe (`SEC_PROBE`, §7.4) is provided for exactly this,
> in the idiom of the existing `TRIM_PROBE`. `TODO.md` §7.3 gates the
> design's acceptance on that measurement.

**Cost model.** One HalfSipHash `SIPROUND` is 4 adds + 4 XORs + 6 rotates;
with no `Zbb` a rotate is `slli`+`srli`+`or`, so a round is ~26 instructions.
Absorbing costs 2 rounds per 4-byte word; finalising costs 6 rounds (the tail
block's 2 plus 4 finalisation rounds).

**Hot loop, one cycle** — 2 × 30 B `GWRITE` + 11 B `COMMIT` ≈ 70 bytes:

| scheme | rounds | ≈ instr | ≈ µs | extra wire |
| ------ | ------ | ------- | ---- | ---------- |
| **stream digest (selected)** | 18 words × 2 + 6 = **42** | 1 090 | **34** | +5 B (one trailer) |
| per-frame MAC | 18 × 2 + 3 × 6 = 54 | 1 400 | 44 | +15 B (three trailers) |
| per-servo digests, N tags in COMMIT | ~10 | 260 | 8 | +33 B for 8 servos |

So the stream digest's real win is **wire**, not CPU: it saves 10 bytes per
cycle (~100 µs at 1 M) against the per-frame scheme, and ~23 % of the MAC
CPU. The per-servo variant is much cheaper in CPU but costs 33 bytes
(~330 µs) of wire per cycle for an 8-servo fleet — the wrong trade on a
protocol whose stated thesis is wire efficiency (§1). CPU here is ~34 µs
against a bus cycle of 1–10 ms; wire is the scarce resource.

**Where it lands.** The fold runs at the **covered checkpoint**, inside the
existing dispatch body, at PFIC HIGH. Its cost therefore behaves like any
other dispatch work:

- **0.5 M / 1 M — hidden.** The covered window is 20–40 µs and the transport
  is grid-bound (`osc-servo-transport.md` §10). Fold work fits under wire time
  and turnaround is unchanged.
- **2 M / 3 M — additive.** These rates are already pipeline-bound; the fold
  serialises after the frame end and adds directly to turnaround.

**Kernel impact.** At a 1 kHz bus cycle, 34 µs is ~3.4 % added transport-HIGH
duty, which by the transport's measured relation (tick loss ≈ 1.2–1.4× HIGH
duty, `osc-servo-transport.md` §2) costs roughly **4–5 % additional kernel
tick coalescing**. Ticks are never starved outright and latency stays bounded,
so the cascade sees a slightly coarser effective sample rate, not a
destabilising one. This is the single number the bench gate must confirm.

If bench shows the cost is material, three tuning levers exist, in order of
preference:

1. **Split feed.** Start the fold at **deadline A** (header readable, 3
   byte-times before the covered checkpoint) over the bytes already ringed,
   finish at the covered checkpoint. This spreads the work across two wakes
   the transport already schedules — no new ISR entries, no new deadlines.
2. **HalfSipHash-1-3** — ~45 % cheaper, same interface.
3. **Policy narrowing** — drop reply-side tags, keep command-side.

### 2.7 Failure behaviour — the control-safety decision

This is the part that most directly touches `control-theory.md`, and it is
deliberately *not* "fail secure" in the naive sense.

**On a MAC failure the servo does NOT cut torque.** A limp control surface on
a UAV is more dangerous than a stuck one — an unpowered surface is driven by
aerodynamic load and can flutter or hard-over, whereas a surface held at its
last commanded position is a known, trimmable disturbance. So:

1. **Revert the staging buffer.** The existing `d.revert()` path. The cascade
   keeps running against the **last authenticated goal**.
2. **Count and alert.** Increment `auth_fail_count`; set the `fault_flags`
   auth bit, which raises **ALERT** on every subsequent status frame (§5.3
   layer 3) so the host learns within one telemetry cycle.
3. **Lock out after `auth_fail_lockout` consecutive failures** (default 3):
   refuse all effect-bearing frames until a successful re-key. This is what
   makes a 32-bit tag sound — online forgery is detected on attempt 3, not
   after 2³¹.
4. **Never** touch the trajectory generator, the estimator layer, the fusion
   filter, or the limits block. The goal simply stops being updated.

**The structural guarantee on control dynamics:** the MAC verdict gates
*staged effects only*. It sits beside the CRC verdict at exactly the same
point in `verify()` (§7.2). It cannot delay a kernel tick by more than its own
compute, it cannot reorder the cascade, and it cannot change a gain, a limit,
or an estimate. The only quantity it can affect is the same one a CRC failure
already affects — whether a staged goal is applied — and the failure action is
identical to a dropped frame, which the control design already tolerates by
construction (the host's timeout-and-retry contract, §3.4).

The one measurable coupling is PFIC HIGH duty, which the transport doc
quantifies as tick loss ≈ 1.2–1.4× transport-HIGH duty
(`osc-servo-transport.md` §2). That is precisely what §2.6's bench gate exists
to bound.

---

## 3. The session plane

Establishment, once per boot, bus-quiet, before the transport starts.

```text
host                                            servo + ECC204
 ──  MGMT SEC_INIT [epoch(2), N_h(16)]  ────────▶
                                                 N_s ← ECC204 RNG (16 B)
                                                 K_uni = HMAC(K_dev,
                                                     "OSC1U" ‖ epoch ‖ N_h ‖ N_s)
 ◀───────  status [N_s(16), serial(9)]  ──
 ──  MGMT SEC_KEY [W(16), tag(16)]  ────────────▶   (group key, wrapped)
                                                 K_grp = W ⊕ HMAC(K_dev,
                                                     "OSC1W" ‖ epoch ‖ N_h)
                                                 verify tag = HMAC(K_dev,
                                                     "OSC1T" ‖ epoch ‖ W)
 ◀───────  status [OK]  ──────────────────
```

- `K_dev` is the ECC204's single symmetric secret slot [REF-SE-001 §1.2]. It
  is provisioned by TrustFLEX/TrustCUSTOM (§5.1) and **never leaves the
  device**.
- `K_uni` (per-servo) and `K_grp` (fleet-wide) are truncated to 8 bytes for
  HalfSipHash. Unicast frames tag with `K_uni`; broadcast frames — `COMMIT`,
  broadcast `GWRITE` — tag with `K_grp`.
- **Why a group key is unavoidable:** the hot loop's apply instant is a single
  broadcast COMMIT on a shared wire (§7). One frame, one tag, N receivers ⇒ a
  shared key. The residual risk is explicit: extracting any one servo's
  `K_grp` from RAM forges broadcasts fleet-wide. It is bounded by the ECC204's
  tamper resistance (JIL "High" [REF-SE-001 §Features]) protecting `K_dev`,
  by `K_grp` being ephemeral RAM-only, and by per-servo `K_uni` still gating
  every unicast and every MGMT operation. Documented, not hidden.
- The KDF is HMAC-SHA-256 executed **inside the certified device**
  [REF-SE-001 §2.1.2, FIPS 198-1], so key derivation is FIPS-approved even
  though the wire tag is not.
- Cost: one `NONCE` plus **three** HMACs (unicast key, wrap pad, wrap-tag
  check) ≈ **670 ms** over SWI — see §0.1 for why an HMAC is two `SHA`
  commands. Once, at boot, with the bus quiet by construction and torque off.
  This is the dominant term in the servo's boot time and must be budgeted for
  in the airframe's power-on sequence.

Re-key (`MGMT SEC_REKEY`) repeats the exchange with a fresh epoch. It requires
a quiet window (§4.3) because it runs SE commands.

---

## 4. Bus-quiet windows

### 4.1 Boot

The natural quiet window: the transport has not started. `main()` runs SE
bring-up **before** `osc_servo_ch32::run!()`. Budget ~200 ms of extra boot
time. Torque is off by construction at boot.

### 4.2 Degraded start

If the SE is absent, unresponsive, or unprovisioned, the servo boots into
**`SecurityState::Unsecured`** and reports it in `status_flags`. It does
**not** refuse to run — a servo that bricks itself on SE failure is a new and
worse failure mode for an aircraft.

Two separate decisions live here, and conflating them is a trap worth naming:

- **State** is `Unsecured` because no session exists yet.
- **Policy** is what decides whether effects are still accepted, and the
  shipped constructor default is **permissive** (`Policy::OPEN`), *not*
  `Policy::FLIGHT`.

The permissive default is deliberate. No ECC204 is fitted on any board in this
repository (`TODO.md` §7.8), so a `require_auth` default would reject every
`COMMIT` on every existing servo — enforcement arriving *ahead* of the
hardware, which is precisely the self-inflicted failure the paragraph above
rejects. A flight build calls `set_policy(Policy::FLIGHT)` explicitly.

> **This is a live risk, not a settled one.** A permissive default means a
> flight build that forgets to set the policy silently accepts unauthenticated
> commands. Making that impossible to forget — a build-time assertion, a
> persisted policy register, or refusing to arm torque under `Policy::OPEN` in
> a flight image — is tracked in `TODO.md` §7.10 and must be closed before the
> design flies.

### 4.3 Runtime quiet window

For re-key and lifecycle grants (§5.3), the host negotiates:

1. Host stops scheduling the bus.
2. Host sends `MGMT SEC_*`; the servo acks **before** starting the SE
   transaction.
3. The servo sets `busy` in `status_flags`, runs the SE exchange (10–520 ms),
   then clears it.
4. Host resumes after observing `busy` clear, or after a SAVE-class timeout.

This reuses `MGMT SAVE`'s existing contract (§9.4): torque provably off, host
uses a long timeout, the servo is genuinely stalled. Nothing new is invented.

---

## 5. The identity plane and provisioning

### 5.1 Provisioning

`K_dev`, the ECC P-256 private key, the device certificate and the CA signer
certificate are installed via Microchip's **TrustFLEX** or **TrustCUSTOM**
flows through the Trust Platform Design Suite and the HSM-backed Secure
Provisioning System [REF-SE-001 §4; REF-SE-004]. `TrustFLEX` is sufficient:
it takes customer-unique credentials into a pre-defined configuration.
`TrustCUSTOM` is required only if the slot configuration must change.

The private key is generated **inside** the device and never exists outside it
[REF-SE-001 §1.3]. The OSC build system never handles it.

### 5.2 Attestation

`MGMT SEC_ATTEST [challenge(32)]` → the ECC204 `SIGN`s
`SHA-256(epoch ‖ challenge ‖ serial)` with its P-256 key; the servo returns
the signature and, on request, the device + CA signer certificates from the SE
EEPROM. The host verifies the chain to its own trust root.

This is the anti-counterfeit / ecosystem-control leg [REF-SE-001 §1.1] and the
only asymmetric operation. ~520 ms, quiet window, boot or on demand.

> **Direction matters.** The ECC204 **signs only — it does not verify**
> [REF-SE-001 §Features: "Hardware Support for the Asymmetric Sign"]. So the
> servo can prove *itself* to the host asymmetrically, but it **cannot** use
> the SE to verify a host's ECDSA signature. Host→servo authentication is
> therefore **symmetric** throughout (the HMAC path, §3) — the ECC204's own
> "Symmetric Authentication through use of an HMAC Key" use case
> [REF-SE-001 §Use Cases]. Software P-256 verification on RV32EC was
> considered and rejected: no hardware multiply-accumulate, ~6–10 KB of flash
> against a 57 KB budget, and hundreds of milliseconds per verification.

### 5.3 Lifecycle grants — the monotonic counter's real job

Per §0.5 the counter has 10 000 counts total. It authorises **rare,
irreversible** operations:

- `MGMT SAVE`, `MGMT FACTORY`, and firmware update require a **grant**: the
  host presents a signed authorisation, the servo increments the ECC204
  counter and binds the grant to the new count, so a captured grant cannot be
  replayed.
- Budget: ~dozens of firmware updates and factory resets over a service life,
  against 10 000 — three orders of margin.
- These operations already require torque-off and a long host timeout (§9.4),
  so the added ~40 ms (`COUNTER` + `SHA`) is free.

---

## 6. Compliance and regulatory posture

### 6.1 Applicability — what actually governs this

US jurisdiction, per project policy.

**TSA Security Directives do not apply to this design.** The TSA cyber
directive series (SD Pipeline-2021-01/02, SD 1580-21-01, SD 1582-21-01, and
the November 2024 NPRM) binds **owners and operators** of individually
designated critical infrastructure — pipelines, freight rail, passenger rail
and transit, and, on the aviation side, airports and aircraft operators. They
impose entity-level obligations: a Cybersecurity Coordinator, a Cyber Risk
Management Program, a CIP/COIP, an IRP, an ADR, a CAP, and 24-hour incident
reporting to CISA.

None of that attaches to a servo actuator, or to the party designing one. TSA
designates covered entities individually and regulates their programmes, not
their component suppliers. An operator who *integrates* this servo into a
designated system may need to treat the aircraft or ground system as a
Critical Cyber System within **their** CRMP — which is a reason to give them
good evidence (§6.3), not a requirement on this repository.

**The airworthiness authority for a UAV flight-control actuator is the FAA,
not TSA.** The applicable framework is:

| Standard | Applies to |
| -------- | ---------- |
| **DO-326A / ED-202A** | Airworthiness Security Process Specification — the governing aviation cyber process |
| **DO-356A / ED-203A** | Airworthiness Security Methods and Considerations — threat/risk assessment method |
| **DO-355 / ED-204** | Information Security Guidance for Continuing Airworthiness |
| **DO-178C** | Airborne software, if a Design Assurance Level is assigned |
| **DO-254** | Airborne electronic hardware |
| **ARP4754B / ARP4761A** | System development and safety assessment |
| **14 CFR Part 107** | Small UAS operations |
| **ASTM F3478** | UAS software development |

> **Status: none of these are yet applied.** This design has been engineered
> to good practice, but no DO-326A security risk assessment, no DAL
> assignment, and no DO-178C objectives evidence exist for this repository.
> That gap is real and is tracked in `TODO.md` §7.7 — it is the difference
> between "defensible engineering" and "certifiable", and this document
> claims only the former.

### 6.2 Cryptographic posture

- **Approved primitives where they are affordable.** ECDSA P-256
  [REF-STD-002], SHA-256 [REF-STD-001], HMAC-SHA-256 [REF-STD-003] and an
  SP 800-90A/B/C certified TRNG [REF-STD-004] — all executed inside the
  ECC204, which offers a FIPS 140-3 compliance-mode configuration bit
  [REF-SE-001 §2.2.3].
- **One documented exception.** HalfSipHash-2-4 as the per-frame tag
  [REF-CRYPTO-001], on a platform where no approved MAC fits the 20 µs budget
  by an order of magnitude (§2.4). Compensating controls: a FIPS-approved KDF
  running inside a certified device, ephemeral keys, epoch+sequence replay
  protection, a 3-failure lockout, and a hardware CRC that retains full
  wire-fault coverage independently.
- **Key management.** Root secrets are HSM-provisioned through Microchip's
  Secure Provisioning System [REF-SE-004] and never leave the tamper-resistant
  device, which carries a JIL "High" attack-potential rating
  [REF-SE-001 §Features].

### 6.3 Export control

The design provides **authentication and integrity only — no confidentiality
service.** No payload is encrypted; the tag is a MAC and the signature is a
digital signature. That matters for classification: **Note 2 to Category 5 —
Part 2 of the EAR Commerce Control List excludes from 5A002/5D002 control
items whose cryptographic functionality is limited to authentication**
(including digital signature and message authentication). On that basis this
firmware is expected to fall **outside** the encryption controls, and no
License Exception ENC notification would be required.

> This is an engineering assessment, not a legal determination. ECCN
> classification is a legal call and must be confirmed by counsel or through a
> CCATS request before export or public distribution — noting the repository
> is already public, which is itself relevant to the analysis (published
> open-source encryption source code has its own treatment under EAR
> §742.15(b)). Tracked in `TODO.md` §7.6.

### 6.4 What an integrator gets

Evidence this design can hand to an operator's own compliance programme:

- A written threat model with explicit in-scope / out-of-scope boundaries (§8).
- Per-device cryptographic identity, attestable on demand (§5.2), which
  supports counterfeit-part detection and supply-chain provenance.
- Tamper-evident telemetry: `auth_fail_count`, `replay_drop_count` and the
  ALERT bit give an operator a detectable, loggable signal that someone is
  talking on the actuator bus who should not be — the actuator-level input to
  an incident-reporting obligation they may hold under their own regime.

---

## 7. Implementation map

### 7.1 New crate — `firmware/lib/osc-security`

`no_std`, host-and-servo shared, no hardware dependencies, unit-testable on
the host. Contents: HalfSipHash-2-4, the KDF label scheme, the staged-write
rolling digest, the replay window, the policy register, and the session state
machine.

### 7.2 The transport hook

The MAC gate is **one condition added beside the CRC verdict**, in
`lib/drivers/src/bus/servo_bus/route.rs::verify()` — the single point where
staged effects are already committed or reverted. This is why the integration
is clean: the transport was already built around "dispatch speculatively, gate
effects on a verdict", which is exactly the structure a per-frame
authenticator needs. The MAC gate is architecturally free; it costs only its
own compute.

### 7.3 The SE driver

`SecureElement` trait (`hmac`, `sign`, `random`, `counter_increment`,
`read_cert`) over a `SecureElementBus` transport, with an I²C implementation
(primary) and a SWI-over-USART2 implementation (alternate). ECC204 command
opcodes and execution times come from [REF-SE-002]; the wire framing and CRC
of the command packet require the NDA-gated full datasheet or the reference
library, and are **not** guessed — see the compile-gate note in §7.5.

### 7.4 Bench probe

`SEC_PROBE` under the existing `bench` feature: MAC cycles per call, bytes
folded, verdict counts, lockout entries. Dumped by symbol address over the
debug link, exactly like `TRIM_PROBE`.

### 7.5 What is deliberately not written

The ECC204 **command-packet framing** (word-address byte, packet CRC-16
parameters, SWI token flag values) is specified only in the NDA-gated full
datasheet. The summary datasheet [REF-SE-001] states this on its cover. Those
constants are therefore **left as a single clearly-marked module that fails
the build if unfilled**, rather than guessed — per this project's authenticity
rule that no reference, constant, or citation is ever fabricated. Everything
above that layer is complete and testable.

---

## 8. Threat model

**In scope.** An attacker with physical access to the three-wire servo bus:
passive observation, frame injection, frame modification, replay, selective
jamming, and connecting a counterfeit servo or a rogue host.

**Out of scope.** Invasive attacks on the ECC204 (bounded by its JIL "High"
rating), compromise of the flight controller itself (it holds the keys by
definition), supply-chain substitution before provisioning, and physical
destruction of the actuator. Confidentiality is **not** a goal: servo
telemetry and position commands are not secret, and encryption would cost the
budget that authenticity needs.

| attack | defence |
| ------ | ------- |
| inject a `goal_position` write | staged digest → COMMIT tag fails → revert (§2.1) |
| inject a non-held write | rejected: `Unauthenticated` (§2.2) |
| replay a captured COMMIT | sequence strictly increasing (§2.5) |
| replay across a power cycle | epoch-bound session key (§2.5) |
| modify a GWRITE slice | digest mismatch → revert (§2.1) |
| suppress a GWRITE (jam) | digest short → revert, fail-safe hold (§2.1) |
| brute-force a tag | 3-failure lockout + ALERT (§2.7) |
| counterfeit servo | ECDSA attestation + cert chain (§5.2) |
| rogue host | symmetric session auth; no `K_dev`, no session (§3) |
| downgrade to unauthenticated | servo-side policy register, not a wire flag (§2.2) |
| wire faults / noise | CRC-16/ARC retained at full strength (§0.2) |
