/* sys_info_v2/gatherer/src/utils.rs
 *
 * Copyright 2023 Romeo Calota
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

pub mod arraystring {
    // pub trait ToArrayStringLossy {
    //     fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY>;
    // }
    //
    // impl ToArrayStringLossy for str {
    //     fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
    //         let mut result = arrayvec::ArrayString::new();
    //         if self.len() > CAPACITY {
    //             for i in (0..CAPACITY).rev() {
    //                 if self.is_char_boundary(i) {
    //                     result.push_str(&self[0..i]);
    //                     break;
    //                 }
    //             }
    //         } else {
    //             result.push_str(self);
    //         }
    //
    //         result
    //     }
    // }
    //
    // impl ToArrayStringLossy for std::borrow::Cow<'_, str> {
    //     fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
    //         let mut result = arrayvec::ArrayString::new();
    //         if self.len() > CAPACITY {
    //             for i in (0..CAPACITY).rev() {
    //                 if self.is_char_boundary(i) {
    //                     result.push_str(&self[0..i]);
    //                     break;
    //                 }
    //             }
    //         } else {
    //             result.push_str(self);
    //         }
    //
    //         result
    //     }
    // }
}
