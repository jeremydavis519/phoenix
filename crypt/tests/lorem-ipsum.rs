/* Copyright (c) 2022 Jeremy Davis (jeremydavis519@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software
 * and associated documentation files (the "Software"), to deal in the Software without restriction,
 * including without limitation the rights to use, copy, modify, merge, publish, distribute,
 * sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or
 * substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
 * NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use crypt::sha512;

#[test]
fn sha512_lorem_ipsum() {
    let bytes = include_bytes!("lorem-ipsum.txt");
    assert_eq!(sha512(bytes), [
        0x0b, 0x7b, 0x28, 0xca, 0x2b, 0xf2, 0x8e, 0x25,
        0x39, 0x29, 0xc8, 0xa2, 0x9d, 0xdb, 0x0a, 0xc2,
        0xa3, 0x92, 0x26, 0xf8, 0x67, 0x02, 0xad, 0x1b,
        0x1e, 0x51, 0x70, 0x3d, 0x5d, 0xce, 0xbd, 0x42,
        0xaf, 0xf7, 0x74, 0x96, 0x9b, 0xb7, 0xe2, 0x3b,
        0xf6, 0xc4, 0x39, 0xba, 0xb4, 0xea, 0xe3, 0x7c,
        0xdf, 0xc8, 0x69, 0x78, 0xa1, 0x76, 0xc2, 0x7e,
        0x83, 0x5c, 0xde, 0xf9, 0xc8, 0xaa, 0xf7, 0xde,
    ]);
}
