fn main() {
    let plain: [[f32; 2]; 3] = [[-1.0, -0.5], [1.0, -0.5], [0.0, 0.5]];
    let flip = |mut x: [[f32; 2]; 3]| { for v in x.iter_mut() { v[1] = -v[1]; } x };
    let pair = |mut x: [[f32; 2]; 3]| { x[1] = x[0]; x };
    let s = [flip(plain), pair(flip(plain))];
    println!("{} (want 0.5)", s[1][0][1]);
}
