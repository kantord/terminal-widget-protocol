# Project tasks.
#
# Docs pipeline: every worked example in RFC.md is ONE JSON source rendered to
# BOTH a fenced code block and an image, so the spec's examples and pictures
# cannot disagree. `mdsh` (https://github.com/zimbatm/mdsh) runs the recipes
# below from HTML-comment directives and inserts their output in place.
#
# Regenerate everything:  just docs
# (Images are regenerated locally and committed; pixel output is font/rasterizer
# dependent, so it is not verified in CI — re-run `just docs` after editing.)

set quiet := true

render := justfile_directory() / "twp-proxy" / "target" / "release" / "twp-render"

# Build the in-process renderer used by the docs pipeline.
_render-bin:
    cargo build --manifest-path twp-proxy/Cargo.toml --release --bin twp-render

# A worked example: the JSON source (shown verbatim) + its rendered image.
#   <!-- > $ just example status-pill 24 3 "Gruvbox" -->
example name cols rows theme="": _render-bin
    {{ render }} --in examples/{{ name }}.json --cols {{ cols }} --rows {{ rows }} {{ if theme != "" { "--theme '" + theme + "'" } else { "" } }} --out docs/figures/{{ name }}.png
    printf '```json\n'
    cat examples/{{ name }}.json
    printf '```\n\n![%s](docs/figures/%s.png)\n' '{{ name }}' '{{ name }}'

# An image-only figure rendered from a registered demo (big scenes whose JSON
# would be noise in the spec).
#   <!-- > $ just figure docker_dashboard_gruvbox_dark docker-dark -->
figure demo out: _render-bin
    {{ render }} --demo {{ demo }} --out docs/figures/{{ out }}.png
    printf '![%s](docs/figures/%s.png)\n' '{{ demo }}' '{{ out }}'

# The §1 hero: the same dashboard scene in two themes, as a side-by-side table.
docker-hero: _render-bin
    {{ render }} --demo docker_dashboard_gruvbox_dark --out docs/figures/docker-dashboard-dark.png
    {{ render }} --demo docker_dashboard_solarized_light --out docs/figures/docker-dashboard-light.png
    printf '| Gruvbox Dark | Solarized Light |\n| --- | --- |\n'
    printf '| ![dashboard in a dark theme](docs/figures/docker-dashboard-dark.png) | ![the same dashboard in a light theme](docs/figures/docker-dashboard-light.png) |\n'

# Regenerate every generated block in RFC.md, in place.
docs: _render-bin
    mdsh --inputs RFC.md

# Rust checks (mirror CI).
check:
    cargo clippy --manifest-path twp-proxy/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path twp-proxy/Cargo.toml --lib
