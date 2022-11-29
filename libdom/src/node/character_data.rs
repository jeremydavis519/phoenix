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

use {
    alloc::rc::Rc,
    core::convert::TryFrom,
    crate::idl::{self, DomString}
};

extern "Rust" {
    // https://dom.spec.whatwg.org/#concept-node-children-changed-ext
    fn run_children_changed_steps(node: Rc<dyn idl::Node>);
}

// https://dom.spec.whatwg.org/#concept-cd-replace
pub(super) fn replace_data<N: idl::CharacterData + ?Sized>(node: Rc<N>, offset: u32, count: u32, data: &Rc<DomString>) {
    // 1. Let length be node’s length.
    let length = node.clone().length();

    // 2. If offset is greater than length, then throw an "IndexSizeError" DOMException.
    if offset > length {
        // TODO
        unimplemented!(r#"Throw an "IndexSizeError" DOMException."#);
    }

    // 3. If offset plus count is greater than length, then set count to length minus offset.
    let count = usize::try_from(u32::min(count, length - offset))
        .expect("replace_data count overflow");

    let offset = usize::try_from(offset)
        .expect("replace_data offset overflow");

    // 4. Queue a mutation record of "characterData" for node with null, null, node’s data, « », « », null, and null.
    // TODO
    todo!();

    // 5. Insert data into node’s data after offset code units.
    // 6. Let delete offset be offset + data’s length.
    // 7. Starting from delete offset code units, remove count code units from node’s data.
    node.clone().data().splice(offset .. offset + count, data.iter().map(|&c| c));

    // 8. For each live range whose start node is node and start offset is greater than offset but less than or equal
    // to offset plus count, set its start offset to offset.
    // TODO
    todo!();

    // 9. For each live range whose end node is node and end offset is greater than offset but less than or equal to
    // offset plus count, set its end offset to offset.
    // TODO
    todo!();

    // 10. For each live range whose start node is node and start offset is greater than offset plus count, increase
    // its start offset by data’s length and decrease it by count.
    // TODO
    todo!();

    // 11. For each live range whose end node is node and end offset is greater than offset plus count, increase its
    // end offset by data’s length and decrease it by count.
    // TODO
    todo!();

    // 12. If node’s parent is non-null, then run the children changed steps for node’s parent.
    if let Some(parent) = node.parentNode() {
        unsafe { run_children_changed_steps(parent); }
    }
}

// https://dom.spec.whatwg.org/#concept-cd-substring
pub(super) fn substring_data<N: idl::CharacterData + ?Sized>(node: Rc<N>, offset: u32, count: u32) -> Rc<DomString> {
    // 1. Let length be node’s length.
    let length = node.clone().length();

    // 2. If offset is greater than length, then throw an "IndexSizeError" DOMException.
    if offset > length {
        // TODO
        unimplemented!(r#"Throw an "IndexSizeError" DOMException."#);
    }

    // 3. If offset plus count is greater than length, return a string whose value is the code units from the offsetth
    // code unit to the end of node’s data, and then return.
    // FIXME: Verify that by "_offsetth_" the standard really means "at zero-based index _offset_".
    if count > length - offset {
        let offset = usize::try_from(offset)
            .expect("substring_data offset overflow");
        return Rc::new(node.data()[offset .. ].to_vec());
    }

    // 4. Return a string whose value is the code units from the offsetth code unit to the offset+countth code unit in
    // node’s data.
    // FIXME: Verify that by "_offsetth_" the standard really means "at zero-based index _offset_".
    let start = usize::try_from(offset)
        .expect("substring_data offset overflow");
    let end = usize::try_from(offset + count)
        .expect("substring_data offset+count overflow");
    Rc::new(node.data()[start .. end].to_vec())
}