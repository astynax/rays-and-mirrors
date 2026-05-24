# Rays and Mirrors

This is a non-interactive demo app that show an animated scene where rays of colored lights are slowly travelling through the field, reflecting from the mirrors, splitting and combining by passing throght prisms. The app works in your terminal in a "full-screen mode" and uses text characters and ANSI escape codes.

## How to

Just `git clone` + `cargo run`!

## Ray colors

The app uses basic ANSI palette made of 16 colors. This makes possible to add two rays of the same dark shade of color and get a single bright ray. Three bright rays of red, green and blue color being combined become a single white ray.

Any two rays crossing each other out of the intersection colored with the same combined color. For the simplicity, if one of the rays is a bright one the combination also receives the bright shade.

## The field

Initially field appears free of any light but containing a few randomly choosen and placed "gadgets" like:

- **light sources** emitting the rays, just a single "pieces" of the light with some color, always appear on the edges of the screen like the light is coming from the ouside.
- **mirrors** either "left upper corner to right down" (\) or "left down corner to right upper" (/).
- **terminators**, absorbing any light from any direction.

## The scene evolution

Under the hood all the lights are pregenerated and always walking from some source up to some terminator or up to the border of the visible area. The app is just animating their propagation one screen character at the time.
