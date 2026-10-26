//! L2 gate: `GetClassName`-family getters return exactly the class-name
//! string constants pinned by `tools/l2/extract_goldens.py` from `game.dat`
//! image bytes (independent oracle: the string bytes at the addresses these
//! functions return).

#[path = "l2/l2_goldens.rs"]
mod goldens;

#[test]
fn l2_class_names_against_image_strings() {
    let call = |va: &str| -> &'static str {
        match va {
            "0x007b3662" => sage_core::classname::class_name_script(),
            "0x007b40fa" => sage_core::classname::class_name_script_group(),
            "0x007b7745" => sage_core::classname::class_name_script_list(),
            "0x007b00d0" => sage_core::classname::class_name_particle_system_storage_module(),
            "0x007b014c" => sage_core::classname::class_name_cpu_particle_system_storage_module(),
            "0x007b03ed" => sage_core::classname::class_name_gpu_particle_system_storage_module(),
            "0x007b040c" => sage_core::classname::class_name_terrain_fire_system_storage_module(),
            "0x007b0e57" => sage_core::classname::class_name_intrinsic_science_purchase_points(),
            "0x007b9124" => sage_core::classname::class_name_living_world_campaign_manager(),
            other => panic!("unmapped classname getter {other}"),
        }
    };
    let mut cases = 0;
    for (va, want) in goldens::CLASSNAME_GOLDENS {
        assert_eq!(call(va), *want, "{va}");
        cases += 1;
    }
    assert_eq!(cases, 9, "all 9 classname getters must be exercised");
}
