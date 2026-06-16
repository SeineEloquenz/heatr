# heat it USB Protocol Specification

Reverse-engineered via Frida instrumentation of the official `de.ka.kamedi.heat_it`
Android app (June 2026). All transfers are USB bulk on ep OUT=`0x02` / IN=`0x82`.

---

## Frame format

All frames share a common envelope:

| Byte | Value | Description |
|------|-------|-------------|
| 0 | `0xFF` | Start-of-frame marker (all requests and responses) |
| 1 | opcode / response-type | See command table below |
| 2..N | payload | Command-specific |

**All responses are exactly 12 bytes.**

3-byte commands follow the pattern `[0xFF, opcode, opcode]` — the opcode is
always repeated as byte 2.

---

## Command reference

| Opcode | Name | Request | Length |
|--------|------|---------|--------|
| `0xB0` | TEST_BOOTLOADER | `ff b0` | 2 |
| `0x02` | GET_STATUS | `ff 02 02` | 3 |
| `0x0E` | GET_DEVICE_INFO | `ff 0e 0e` | 3 |
| `0x0D` | READ_MEMORY | `ff 0d addr_hi addr_lo 06 cksum` | 6 |
| `0x32` | POLL | `ff 32 32` | 3 |
| `0x08` | MSG_START_HEATING | `ff 08 gen_skin dur cksum` | 5 |
| `0x18` | STOP_HEATING | `ff 18 18` | 3 |

---

## Checksum formulas

### Request checksum (READ_MEMORY and MSG_START_HEATING)

```
cksum = sum(bytes[1..=N-2]) % 256
```

That is: sum all bytes after the `0xFF` header up to (but not including) the
checksum byte itself, truncated to 8 bits.

Verified examples:

| Request | Sum of bytes[1..] | Cksum |
|---------|-------------------|-------|
| `ff 0d f8 8e 06` | `0x0d+0xf8+0x8e+0x06 = 0x199` | `0x99` ✓ |
| `ff 0d ff cc 06` | `0x0d+0xff+0xcc+0x06 = 0x1de` | `0xde` ✓ |
| `ff 08 00 00` | `0x08+0x00+0x00 = 0x08` | `0x08` ✓ |
| `ff 08 02 01` | `0x08+0x02+0x01 = 0x0b` | `0x0b` ✓ |

### Response checksum (GET_STATUS and POLL)

```
cksum = sum(bytes[1..=7]) % 256   →  stored in byte[8]
```

Bytes `[9..=11]` are a fixed footer `ff ff 4f`.

Verified examples:

| bytes[1..=7] | Expected cksum (byte[8]) |
|--------------|--------------------------|
| `00 01 64 00 00 00 00` | `0x65` ✓ |
| `00 01 64 80 01 07 f8` | `0xe5` ✓ |
| `00 01 97 80 01 07 f8` | `0x18` ✓ |

### Response checksum (READ_MEMORY)

```
cksum = sum(bytes[1..=10]) % 256  →  stored in byte[11]
```

(i.e., covers the full echo header + 6 data bytes.)

---
## GET_STATUS response fields

```
[0]  ff      SOF
[1]  00      response type (always 0x00)

[2]  temp_hi \
[3]  temp_lo /  temperature-related measurement

[4]  flags
               0x80 = heating element enabled
               0x00 = heating element disabled

[5]  phase
               0x00 = idle / finished
               0x01 = active heating / ramp-up
               0x02 = regulation / hold phase

[6]  ctrl_a   control-loop value (purpose unknown)
[7]  ctrl_b   control-loop value (purpose unknown)

[8]  cksum    sum(bytes[1..=7]) % 256

[9]  ff
[10] ff
[11] 4f
```

### Tentative Celsius interpretation

The device manufacturer advertises treatment temperatures of approximately:

```
47°C – 52°C
```

The observed peak values:

```
0x01FA = 506
0x0206 = 518
```

would correspond to:

```
50.6°C
51.8°C
```

if the field is interpreted as temperature in tenths of a degree Celsius.

Therefore the current best hypothesis is:

```rust
let temperature_raw =
    ((packet[2] as u16) << 8) | packet[3] as u16;

let temperature_celsius =
    temperature_raw as f32 / 10.0;
```

This interpretation matches:

* the observed heating ramp,
* the observed regulation around 51–52°C,
* and the manufacturer's published temperature range.

---

## POLL response fields

`POLL` mirrors the most recent `GET_STATUS` payload but echoes the command in
bytes `[0..=2]`:

```
[0]  ff
[1]  32
[2]  32  (or 0x31 when device state has changed since last POLL)
[3..11]  identical to bytes[3..=11] of the last GET_STATUS response
```

---

## GET_DEVICE_INFO response fields

```
[0]  ff
[1]  0e      command echo
[2]  base1_hi  \  start address of firmware config region
[3]  base1_lo  /  (observed: 0xF88E)
[4]  base2_hi  \  start address of serial-number region
[5]  base2_lo  /  (observed: 0xF8C0)
[6..10]  additional device metadata (firmware rev, build date…)
[11] 00      terminator
```

The host uses `base1` and `base2` to drive the READ_MEMORY sequence below.

---

## READ_MEMORY request / response

**Request:** `ff 0d addr_hi addr_lo 06 cksum` — always reads 6 bytes.

**Response:**
```
[0]     ff
[1]     0d       command echo
[2..3]  addr     address echo (big-endian)
[4]     06       length echo
[5..10] data     6 data bytes from device memory
[11]    cksum    sum(bytes[1..=10]) % 256
```

---

## MSG_START_HEATING encoding

```
gen_skin = (generation_code << 1) | skin_sensitivity_code
  generation:      child=0, adult=1
  skin_sensitivity: sensitive=0, regular=1

dur = duration_code
  short=0, medium=1, long=2

request = [0xFF, 0x08, gen_skin, dur, (0x08 + gen_skin + dur) % 256]
```

---

## Device memory layout (from READ_MEMORY trace)

Three memory regions are read during initialization:

### Region A — firmware config (`base1`, 2 chunks of 6)

| Address | Observed data | Notes |
|---------|---------------|-------|
| `0xF88E` | `01 00 00 02 01 01` | firmware config / version flags |
| `0xF894` | `00 01 01 04 00 ff` | continuation |

### Region B — unique ID (`0xFFC0`, 3 chunks of 6, hardcoded)

| Address | Observed data | Notes |
|---------|---------------|-------|
| `0xFFC0` | `82 27 8e 8a 86 43` | unique device ID / hash |
| `0xFFC6` | `ef 11 a4 81 64 43` | |
| `0xFFCC` | `2b cc 54 66 ff ff` | |

### Region C — serial number (`base2`, 3 chunks of 6)

| Address | Observed data | Notes |
|---------|---------------|-------|
| `0xF8C0` | `34 32 36 32 33 37` = `"426237"` | serial number (ASCII) |
| `0xF8C6` | `30 32 39 30 32 33` = `"029023"` | serial number cont. |
| `0xF8CC` | `38 44 ff ff ff ff` = `"8D"` + pad | serial number tail |

Full serial number: `"4262370290238D"`

---

## Required session sequences

### Initialization (must complete before heating)

```
→ ff b0                           TEST_BOOTLOADER
← 12-byte response

→ ff 0e 0e                        GET_DEVICE_INFO  (extract base1, base2)
← 12-byte response

→ ff 0d base1_hi  base1_lo  06 cksum    READ_MEMORY base1
← 12-byte response
→ ff 0d base1_hi' base1_lo' 06 cksum   READ_MEMORY base1+6
← 12-byte response

→ ff 0d ff c0 06 d2               READ_MEMORY 0xFFC0  (hardcoded)
← 12-byte response
→ ff 0d ff c6 06 d8               READ_MEMORY 0xFFC6
← 12-byte response
→ ff 0d ff cc 06 de               READ_MEMORY 0xFFCC
← 12-byte response

→ ff 0d base2_hi  base2_lo  06 cksum   READ_MEMORY base2
← 12-byte response
→ ff 0d base2_hi' base2_lo' 06 cksum  READ_MEMORY base2+6
← 12-byte response
→ ff 0d base2_hi" base2_lo" 06 cksum  READ_MEMORY base2+12
← 12-byte response

→ ff 02 02                        GET_STATUS
← 12-byte response
→ ff 32 32                        POLL
← 12-byte response
```

### Heating

```
→ ff 02 02                        GET_STATUS  (preflight)
← 12-byte response
→ ff 32 32                        POLL
← 12-byte response

→ ff 08 gen_skin dur cksum        MSG_START_HEATING
← 12-byte response

[repeat until done:]
→ ff 02 02                        GET_STATUS
← 12-byte response  (byte[4] flags: 0x80=heating, 0x00=done)
→ ff 32 32                        POLL
← 12-byte response

→ ff 18 18                        STOP_HEATING
← 12-byte response  (byte[1] = 0xf1 = acknowledged)
```

`POLL` must always immediately follow `GET_STATUS`. The device appears to
maintain a state machine that expects the paired sequence.

---

## Notes on STOP_HEATING

In the trace the app sent `ff 18 18` four times in succession before the device
responded with `ff f1 18 ...` (byte[1] = `0xf1`). The `0xf1` response code also
appears in the `TEST_BOOTLOADER` response — it seems to indicate a transition
into/out of a special device state rather than a simple ACK.
