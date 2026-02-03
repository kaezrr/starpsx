use super::*;

// impl MultiplyAccumVector {
//     /// Add a vec3 to the accumulators
//     pub fn add_vec(&mut self, v: Vector3) {
//         self.raw1 += v.x.0 as i64;
//         self.raw2 += v.y.0 as i64;
//         self.raw3 += v.z.0 as i64;
//     }
//
//     /// Multiply a matrix and a vector and add them to the accumulator
//     pub fn mul_mat_vec(&mut self, m: Matrix3, v: Vector3) {
//         self.raw1 = (m.elems[0] * v.x) + (m.elems[1] * v.y) + (m.elems[2] * v.z);
//         self.raw2 = (m.elems[3] * v.x) + (m.elems[4] * v.y) + (m.elems[5] * v.z);
//         self.raw3 = (m.elems[6] * v.x) + (m.elems[7] * v.y) + (m.elems[8] * v.z);
//     }
//
//     /// Arithmetic shift right
//     pub fn shr(&mut self, sf: usize) {
//         self.raw1 >>= sf;
//         self.raw2 >>= sf;
//         self.raw3 >>= sf;
//     }
// }
