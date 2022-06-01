# TopCodes in Rust

This is a direct reimplementation of TopCodes in Rust. The original source by
Michael Horn can be found [here](https://github.com/TIDAL-Lab/TopCodes).

## Thresholding

For a peak into how the scanner works, we start with an image such as the
following:

<img src="assets/photo.png" width="25%"/>

It runs the thresholding algorithm which produces the following data in the
alpha channel (visualized as a greyscale image):

<img src="assets/after_thresholding_alpha_only.png" width="25%"/>

## Scanning

After the thresholding, TopCodes are determined from this black and white map.
First the candidate TopCode is ensured not to overlap existing TopCodes
(opportunity for a BVH or similar data structure to determine collisions
quickly), then unit size (width of ring) is determined, and finally the actual
code is determined. There is a checksum to ensure that the code's number of 1's
bits is equal to five to reduce the number of valid TopCodes (and thus decrease
the error rate).

## Plans

The goal of this package is to be as agnostic of the platform as possible. All
dependencies that are not explicitly required will be feature-gated to ensure
that the default dependencies of this project are as close to zero as possible.
Ideally, this version of the project should be able to run on most/all
platforms that are supported by Rust out of the box.

I plan to create a separate repository for providing a dynamic library from this
source, so that it can be pulled in from other languages, as well.
