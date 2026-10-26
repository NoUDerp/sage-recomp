//! L2 gate for the 13 DX render-state enum→name getters
//! (rows `0x0051ee20 0x0051ee50 0x0051ee80 0x0051eeb0 0x0051ef50 0x0051ef80
//! 0x0051eff0 0x0051f030 0x0051f0a0 0x0051f0d0 0x0051f120 0x0051f140
//! 0x0051f160`). Case structure from the decompiled bodies, name strings from
//! the game image bytes (`goldens`, sha256-keyed).

#[path = "l2/l2_goldens.rs"]
mod goldens;

use sage_core::dxstate;

#[test]
fn l2_dx_enum_names_against_image_strings() {
    let call = |va: &str, input: u32| -> &'static str {
        match va {
            "0x0051ee20" => dxstate::zbuffer_type_name(input as i32),
            "0x0051ee50" => dxstate::fill_mode_name(input as i32),
            "0x0051ee80" => dxstate::shade_mode_name(input as i32),
            "0x0051eeb0" => dxstate::blend_func_name(input),
            "0x0051ef50" => dxstate::cull_mode_name(input as i32),
            "0x0051ef80" => dxstate::compare_func_name(input),
            "0x0051eff0" => dxstate::fog_mode_name(input),
            "0x0051f030" => dxstate::stencil_op_name(input),
            "0x0051f0a0" => dxstate::material_color_source_name(input as i32),
            "0x0051f0d0" => dxstate::vertex_blend_mode_name(input),
            "0x0051f120" => dxstate::patch_edge_style_name(input as i32),
            "0x0051f140" => dxstate::disabled_material_mode_name(input as i32),
            "0x0051f160" => dxstate::blend_op_name(input),
            other => panic!("unmapped getter row {other}"),
        }
    };
    let mut cases: std::collections::HashMap<&str, Vec<u32>> = std::collections::HashMap::new();
    for (va, input, want) in goldens::DX_ENUM_NAMES {
        assert_eq!(call(va, *input), *want, "{va} input {input}");
        cases.entry(va).or_default().push(*input);
    }
    assert_eq!(cases.len(), 13, "all 13 getters must be exercised");
    // Every getter falls back to the image blob `">UNKNOWN"` + 1
    // (0x00bcfe08) for inputs outside its golden case set.
    for (va, inputs) in &cases {
        let mut probes = 0;
        for probe in [u32::MAX, 0x0000_1234] {
            if !inputs.contains(&probe) {
                assert_eq!(call(va, probe), goldens::DX_UNKNOWN_NAME, "{va} probe {probe:#x}");
                probes += 1;
            }
        }
        assert_eq!(probes, 2, "probe set must stay disjoint from cases");
    }
}
