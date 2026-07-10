use tauri::{Manager, PhysicalPosition};

fn test(app: &tauri::AppHandle) {
    if let Ok(cursor) = app.cursor_position() {
        println!("cursor: {:?}", cursor);
    }
}
