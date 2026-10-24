External shaders live in Common/, GBA/, GB/ or GBC/.

Common/ is offered to every platform. GBA/, GB/ and GBC/ only appear for
their own platform; GBC does not read GB/.

Each entry is a RetroArch GLSL preset (.glslp) or a RetroArch vertex/fragment
source (.glsl). slot currently loads single-pass presets only.

linear is a minimal bilinear pass with no sharpening or other processing.
It is useful as a reference for how presets and filter_linear0 work.

sharp-shimmerless and sharp-shimmerless-grid are by zadpos and are public
domain:
https://github.com/Woohyun-Kang/Sharp-Shimmerless-Shader

The grid variant's GRID_RATIO_X/Y and GRID_OPACITY_X/Y parameters use the
upstream defaults (0.3). slot currently compiles them as constants rather
than exposing them in the menu.
