# heatr

A free and open source app for interfacing with heat-based USB insect bite healers (e.g., the **heat it®** by Kamedi GmbH).
Inspired by [itchcraft](https://github.com/claui/itchcraft).

> ⚠️ **IMPORTANT SAFETY NOTICE:** This is NOT A CERTIFIED MEDICAL PRODUCT.
> We **ARE NOT LIABLE** for any damage you do to yourself using this.

---

## Building

```bash
# Build everything
cargo build --release

# Run tests (no physical device required)
cargo test

# Run the CLI
./target/release/heatr --help
./target/release/heatr info
./target/release/heatr start --duration short --generation child --skin-sensitivity sensitive
```

### Linux udev rules

Without udev rules, opening the device requires root. Copy the rule file:

```bash
sudo cp udev/60-heatr.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

---

## Using the library from Rust

```toml
[dependencies]
heatr = { path = "crates/heatr" }
```

```rust
use heatr::{Api, Preferences, Duration, Generation, SkinSensitivity};

let api = Api::new();

for healer in api.info().await? {
    println!("{} – {}", healer.product_name(), healer.vendor_name());
}

api.start(Preferences {
    duration: Duration::Short,
    generation: Generation::Child,
    skin_sensitivity: SkinSensitivity::Sensitive,
}, |_| {}).await?;
```
