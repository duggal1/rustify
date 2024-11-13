pub fn create_framework_specific_config(app_type: &str) -> io::Result<()> {
    match app_type {
        "nextjs" => create_nextjs_optimized_config()?,
        "react" => create_react_optimized_config()?,
        "vue" => create_vue_optimized_config()?,
        "svelte" => create_svelte_optimized_config()?,
        "angular" => create_angular_optimized_config()?,
        "astro" => create_astro_optimized_config()?,
        "remix" => create_remix_optimized_config()?,
        "mern" => create_mern_optimized_config()?,
        _ => return Ok(()),
    }
    Ok(())
} 