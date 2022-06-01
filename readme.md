# bevy_slyedoc_bvh

[![Watch the video](./docs/bvh_render.mp4)]

Credit: This is largely based on the amazing [tutorial](https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/) series by Jacco Bikker.  Go check it out.

This is very much work in progress and early days.

## Goal

Test if I can build a performant bounding volume hierarchy directly in bevy.

And to test it use it as a CPU based raycast renderer.

Warning: At the moment I wouldn't leave this running.

## Notes

I am leaving a lot of Jacco's optimization out at the moment, first focusing on getting everything working and readability.  For example, he doesn't use Vectors at all.

The examples are more used to help me test and debug at the moment.