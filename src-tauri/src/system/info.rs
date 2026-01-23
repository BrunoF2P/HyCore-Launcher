use sysinfo::System;

#[tauri::command]
pub fn get_system_ram_gb() -> u32 {
    get_total_ram_gb_internal()
}

pub fn get_total_ram_gb_internal() -> u32 {
    let mut sys = System::new();
    sys.refresh_memory();
    let total_memory_bytes = sys.total_memory();
    let total_memory_gb = (total_memory_bytes / 1024 / 1024 / 1024) as u32;
    total_memory_gb
}
