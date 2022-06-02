# bevy_slyedoc_bvh

// TODO: Make better vid
![image](./docs/bvh_render.gif)

Credit: This is largely based on the amazing [tutorial](https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/) series by Jacco Bikker.  Go check it out.

This is very much work in progress and early days.

## Goal

Test if I can build a performant bounding volume hierarchy directly in bevy.

And to test it out, use it for CPU raycast renderer.

## Notes

I am leaving a lot of Jacco's optimization out at the moment, first focusing on getting everything working and reusable.  For example, he doesn't use Vectors at all to avoid allocating memory.  

Also I am not using simd yet, want to learn glam better, also avoiding unions.

The examples are more used to help me test and debug at the moment. For example use case checkout the [Cursor Plugin](./examples/helpers/cursor.rs)
