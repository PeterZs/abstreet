#version 140

// (x offset, y offset, zoom)
uniform vec3 transform;
// (window width, window height)
uniform vec2 window;

in vec2 position;
in vec4 color;
out vec4 pass_color;

void main() {
    pass_color = color / 255.0;

    // This is map_to_screen
    float screen_x = (position[0] * transform[2]) - transform[0];
    float screen_y = (position[1] * transform[2]) - transform[1];
    // Translate that to clip-space or whatever it's called
    float x = (screen_x / window[0] * 2.0) - 1;
    float y = (screen_y / window[1] * 2.0) - 1;
    // Note the y inversion
    gl_Position = vec4(x, -y, 0.0, 1.0);
}
