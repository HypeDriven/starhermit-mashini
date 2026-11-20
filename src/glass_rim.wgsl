// A narrow polished edge broadens a reflected highlight. Approximate its
// diffraction envelope at red, green and blue wavelengths (micrometres).
fn fragment(f: Fragment) -> vec4f {
    let eye = normalize(camera_position() - f.world);
    let normal = normalize(f.normal);
    let n = select(-normal, normal, dot(normal, eye) >= 0.0);
    let light = normalize(vec3f(-13.0, 24.0, 12.0));
    let mirror = 2.0 * dot(n, light) * n - light;
    let error = length(eye - mirror);
    let phase = max(vec3f(0.001), 3.14159 * 1.8 * error / vec3f(0.650, 0.535, 0.455));
    let wave = sin(phase) / phase;
    let bands = wave * wave * max(dot(n, light), 0.0);
    let fresnel = 0.043 + 0.957 * pow(1.0 - max(dot(eye, n), 0.0), 5.0);
    return vec4f(vec3f(0.72, 0.91, 0.91) + 3.0 * bands,
                 clamp(0.15 + 0.60 * fresnel + dot(bands, vec3f(0.45)), 0.0, 0.98));
}
