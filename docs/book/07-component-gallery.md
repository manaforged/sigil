# Component gallery

This gallery is the image library for Sigil documentation and the future
`sigil-ui.com` website. Each image comes from a real Sigil widget rendered
through the offscreen wgpu path at 2× scale.

Run this command from the repository root to refresh every image:

```sh
cargo test -p sigil-ui --test catalog -- --ignored --test-threads=1
```

Review every changed PNG before you commit it. These images show the current
component output. They are not hand-built mockups.

## Light theme

### Buttons

![Default button in the light theme](assets/components/light/button.png)

![Primary button in the light theme](assets/components/light/button_primary.png)

### Status and progress

![Status chips in the light theme](assets/components/light/chips.png)

![Gauge in the light theme](assets/components/light/gauge.png)

![Progress bar in the light theme](assets/components/light/progress.png)

### Form controls

![Dropdown in the light theme](assets/components/light/dropdown.png)

![Radio group in the light theme](assets/components/light/radio_group.png)

![Checkbox in the light theme](assets/components/light/checkbox.png)

![Slider in the light theme](assets/components/light/slider.png)

![Text field in the light theme](assets/components/light/text_field.png)

![Toggle in the light theme](assets/components/light/toggle.png)

![Tabs in the light theme](assets/components/light/tabs.png)

## Dark theme

### Buttons

![Default button in the dark theme](assets/components/dark/button.png)

![Primary button in the dark theme](assets/components/dark/button_primary.png)

### Status and progress

![Status chips in the dark theme](assets/components/dark/chips.png)

![Gauge in the dark theme](assets/components/dark/gauge.png)

![Progress bar in the dark theme](assets/components/dark/progress.png)

### Form controls

![Dropdown in the dark theme](assets/components/dark/dropdown.png)

![Radio group in the dark theme](assets/components/dark/radio_group.png)

![Checkbox in the dark theme](assets/components/dark/checkbox.png)

![Slider in the dark theme](assets/components/dark/slider.png)

![Text field in the dark theme](assets/components/dark/text_field.png)

![Toggle in the dark theme](assets/components/dark/toggle.png)

![Tabs in the dark theme](assets/components/dark/tabs.png)

The capture definitions live in `crates/sigil/tests/catalog.rs`. Add a new
entry there when a component is ready for the public gallery. Keep the output
name stable so documentation and website links do not break.
