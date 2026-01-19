use sysinfo::System;

#[tauri::command]
pub fn get_system_ram_gb() -> u32 {
    let mut sys = System::new_all();
    sys.refresh_memory();
    let total_memory_bytes = sys.total_memory();
    // sysinfo 0.37+ returns memory in bytes, convert to GB
    let total_memory_gb = (total_memory_bytes / 1024 / 1024 / 1024) as u32;

    log::info!(
        "System total RAM: {} GB (from {} bytes)",
        total_memory_gb,
        total_memory_bytes
    );
    total_memory_gb
}
