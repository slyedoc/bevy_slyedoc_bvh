# Bevy_Slyedoc_Bvh

A Bevy Plugin for bounding volume hierarchy.

This project is just an experiment for my own enjoyment at the moment.

## Credit

This is largely based on the amazing [tutorial series](https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/) by Jacco Bikker.  Go check it out if bvh's interest you.

## Context

Any *production* bevy raytracing solution would most likely be based on [VK_KHR_ray_tracing_pipeline](https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VK_KHR_ray_tracing_pipeline.html) assuming you want to take advantage of the latest GPU hardware.  That would require deep knowledge of vulkan, wgpu, and bevy.  This is not that project, not yet at least.

How does that hardware help?

> RTX cards feature fixed-function "RT cores" that are designed to accelerate mathematical operations needed to simulate rays, such as bounding volume hierarchy traversal.  - [wikipedia](https://en.wikipedia.org/wiki/Nvidia_RTX#Ray_tracing)

These cores speed up the BVH traversal, but the acceleration structure is computed on the cpu.  This project is an experiment to ask how useful a bvh is even without using it for rendering would be in bevy.  Of coarse if you have a bvh, your going to try and rendering with it.

## Overview

This is broke up into 2 crates:
- bvh: plugin produces a bvh bevy resource that can be use from any system.
  - Has a cpu based camera for stress testing and debugging
  
- raytrace: an attempt to send the bvh to a shader and rendering with it.  I am new to shaders and this is nowhere near ideal and I wouldn't copy anything from this if I was you.  

## Notes

Currently we are duplicating mesh data at the moment and rebuilding the bvh each frame.  Will add refit later.
## Other Resources

- [NVIDIA Raytrace Tutoral](https://developer.nvidia.com/rtx/raytracing/vkray) This is c++ and for the vulkan extention, and it was pretty rough to get though
