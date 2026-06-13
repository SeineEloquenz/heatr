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

Without udev rules, opening the device requires root. This applies to both the
CLI and the desktop app. Copy the rule file:

```bash
sudo cp nix/udev/60-itchcraft.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

---

## Desktop app (GTK)

`heatr-gtk` is a native GTK4 / libadwaita client. Plug in a supported device,
choose your settings, and run a heating session from a graphical UI.

It needs the GTK4 and libadwaita system libraries. The dev shell provides them:

```bash
nix develop # drops you into a shell with gtk4 + libadwaita

# Run from source
cargo run -p heatr-gtk

# Run against a simulated device — no hardware required.
cargo run -p heatr-gtk --features mock-device
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
