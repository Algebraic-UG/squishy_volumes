// This is a WGSL port of https://github.com/wi-re/tbtSVD/blob/master/source/SVD.h (MIT license)

// which is an implementation of "Computing the Singular Value Decomposition of 3x3 matrices with minimal branching and
// elementary floating point operations" from http://pages.cs.wisc.edu/~sifakis/project_pages/svd.html

/// A unit quaternion representing a rotation.
struct Quat {
    /// The quaternion’s coordinates (i, j, k, w).
    coords: vec4<f32>,
}

/// The quaternion representing an identity rotation.
fn identity() -> Quat {
    return Quat(vec4(0.0, 0.0, 0.0, 1.0));
}

/// Convert an axis-angle (represented as the axis multiplied by the angle) to
/// a quaternion.
fn fromScaledAxis(axisangle: vec3<f32>) -> Quat {
    let angle = length(axisangle);
    let is_zero = f32(angle == 0.0);

    if angle == 0.0 {
        return identity();
    } else {
        let hs = sin(angle / 2.0);
        let hc = cos(angle / 2.0);
        let axis = axisangle / angle;
        return Quat(vec4(axis * hs, hc));
    }
}

// Converts this quaternion to a rotation matrix.
fn toMatrix(quat: Quat) -> mat3x3<f32> {
    let i = quat.coords.x;
    let j = quat.coords.y;
    let k = quat.coords.z;
    let w = quat.coords.w;

    let ww = w * w;
    let ii = i * i;
    let jj = j * j;
    let kk = k * k;
    let ij = i * j * 2.0;
    let wk = w * k * 2.0;
    let wj = w * j * 2.0;
    let ik = i * k * 2.0;
    let jk = j * k * 2.0;
    let wi = w * i * 2.0;

    return mat3x3(
        vec3(ww + ii - jj - kk, wk + ij, ik - wj),
        vec3(ij - wk, ww - ii + jj - kk, wi + jk),
        vec3(wj + ik, jk - wi, ww - ii - jj + kk),
    );
}

/// Normalizes this quaternion again using a first-order Taylor approximation.
/// This is useful when repeated computations might cause a drift in the norm
/// because of float inaccuracies.
fn renormalizeFast(q: Quat) -> Quat {
    let sq_norm = dot(q.coords, q.coords);
    return Quat(q.coords * (0.5 * (3.0 - sq_norm)));
}

fn imag(q: Quat) -> vec3<f32> {
    return q.coords.xyz;
}

/// The inverse (conjugate) of a unit quaternion.
fn inv(q: Quat) -> Quat {
    return Quat(vec4(-q.coords.xyz, q.coords.w));
}

/// Multiplies two quaternions (combines their rotations).
fn mul(lhs: Quat, rhs: Quat) -> Quat {
    let scalar = lhs.coords.w * rhs.coords.w - dot(lhs.coords.xyz, rhs.coords.xyz);
    let v = cross(lhs.coords.xyz, rhs.coords.xyz) + lhs.coords.w * rhs.coords.xyz + rhs.coords.w * lhs.coords.xyz;
    return Quat(vec4(v, scalar));
}

/// Multiplies a quaternion by a vector (rotates the vector).
fn mulVec(q: Quat, v: vec3<f32>) -> vec3<f32> {
    let t = cross(q.coords.xyz, v) * 2.0;
    let c = cross(q.coords.xyz, t);
    return t * q.coords.w + c + v;
}

/// Multiplies a quaternion’s inverse by a vector (inverse-rotates the vector).
fn invMulVec(q: Quat, v: vec3<f32>) -> vec3<f32> {
    let t = cross(q.coords.xyz, v) * 2.0;
    let c = cross(q.coords.xyz, t);
    return t * -q.coords.w + c + v;
}

fn diff_conj1_2(a: Quat, b: Quat) -> mat3x3<f32> {
    let v1 = imag(a);
    let v2 = imag(b);
    let w1 = a.coords.w;
    let w2 = b.coords.w;

    // TODO: this can probably be optimized a lot by unrolling the ops.
    return (tensor_prod(v1, v2) + diag(w1 * w2)
            - cross_matrix(v1 * w2 + v2 * w1)
            + cross_matrix(v1) * cross_matrix(v2))
            * 0.5;
}

/*
 * Misc utilities.
 */
fn tensor_prod(a: vec3<f32>, b: vec3<f32>) -> mat3x3<f32> {
    // NOTE: keep in mind that the matrix is initialized column
    //       by column. So the element order look transposed compared
    //       to traditional mathematical notation.
    return mat3x3(
        vec3(a.x * b.x, a.y * b.x, a.z * b.x),
        vec3(a.x * b.y, a.y * b.y, a.z * b.y),
        vec3(a.x * b.z, a.y * b.z, a.z * b.z),
    );
}

fn diag(elt: f32) -> mat3x3<f32> {
    return mat3x3(
        vec3(elt, 0.0, 0.0),
        vec3(0.0, elt, 0.0),
        vec3(0.0, 0.0, elt),
    );
}

fn cross_matrix(r: vec3<f32>) -> mat3x3<f32> {
    // NOTE: keep in mind that the matrix is initialized column
    //       by column. So the element order look transposed compared
    //       to traditional mathematical notation.
    return mat3x3(
        vec3(0.0, r.z, -r.y),
        vec3(-r.z, 0.0, r.x),
        vec3(r.y, -r.x, 0.0),
    );
}

// The SVD of a 3x3 matrix.
struct Svd {
    U: mat3x3<f32>,
    S: vec3<f32>,
    Vt: mat3x3<f32>,
};

// Constants used for calculation of givens quaternions
const GAMMA: f32 = 5.828427124; // sqrt(8)+3;
const CSTAR: f32 = 0.923879532; // cos(pi/8)
const SSTAR: f32 = 0.3826834323; // sin(p/8)
// Threshold value
const SVD_EPSILON: f32 = 1e-6;
// Iteration counts for Jacobi Eigenanlysis and reciprocal square root functions, influence precision
const JACOBI_STEPS: u32 = 12;
const RSQRT_STEPS: u32 = 4;
const RSQRT1_STEPS: u32 = 6;

// The QR decomposition of a 3x3 matrix.
struct QR {
    Q: mat3x3<f32>,
    R: mat3x3<f32>,
};

// A simple symmetrix 3x3 Matrix class (contains no storage for (0, 1) (0, 2) and (1, 2)
struct Symmetric3x3 {
    // TODO: for some reasons, naming these m00, m10, etc. doesn’t compile when
    //       using svd3 as a module from another shader.
    mxx: f32,
    myx: f32,
    myy: f32,
    mzx: f32,
    mzy: f32,
    mzz: f32,
}

// Helper struct to store 2 floats to avoid OUT parameters on functions
struct givens {
    ch: f32,
    sh: f32,
};

/// Calculates the reciprocal square root of x using a fast approximation.
/// The number of newton iterations can be controlled using RSQRT_STEPS.
// A built-in rsqrt function or 1.f/sqrt(x) could be used, however doing this manually allows for exact matching results on CPU and GPU code.
fn rsqrt(val: f32) -> f32 {
    var x = val;
    let xhalf = -0.5f * x;
    var i = bitcast<i32>(x);
    i = 0x5f375a82 - (i >> 1);
    x = bitcast<f32>(i);
    for (var i = 0u; i < RSQRT_STEPS; i++) {
        x = x * fma(x * x, xhalf, 1.5f);
    }
    return x;
}

// See rsqrt. Uses RSQRT1_STEPS to offer a higher precision alternative
fn rsqrt1(val: f32) -> f32 {
    var x = val;
    let xhalf = -0.5f * x;
    var i = bitcast<i32>(x);
    i = 0x5f375a82 - (i >> 1);
    x = bitcast<f32>(i);
    for (var i = 0u; i < RSQRT1_STEPS; i++) {
        x = x * fma(x * x, xhalf, 1.5f);
    }
    return x;
}

// Calculates the square root of x using 1.f/rsqrt1(x).
fn accurateSqrt(x: f32) -> f32 {
    return 1.f / rsqrt1(x);
}

// Helper function used to swap X with Y and Y with  X if c == true
fn condSwap(c: bool, x: ptr<function, f32>, y: ptr<function, f32>) {
    let x0 = *x;
    *x = select(*x, *y, c);
    *y = select(*y, x0, c);
}

// Helper function used to swap X with Y and Y with -X if c == true
fn condNegSwap(c: bool, x: ptr<function, f32>, y: ptr<function, f32>) {
    let x0 = -*x;
    *x = select(*x, *y, c);
    *y = select(*y, x0, c);
}

fn condSwapVec(c: bool, x: ptr<function, vec3<f32>>, y: ptr<function, vec3<f32>>) {
    let x0 = *x;
    *x = select(*x, *y, vec3(c));
    *y = select(*y, x0, vec3(c));
}

fn condNegSwapVec(c: bool, x: ptr<function, vec3<f32>>, y: ptr<function, vec3<f32>>) {
    let x0 = -*x;
    *x = select(*x, *y, vec3(c));
    *y = select(*y, x0, vec3(c));
}

// For an explanation of the math see http://pages.cs.wisc.edu/~sifakis/papers/SVD_TR1690.pdf
// Computing the Singular Value Decomposition of 3 x 3 matrices with minimal branching and elementary floating point operations
// See Algorithm 2 in reference. Given a matrix A this function returns the givens quaternion (x and w component, y and z are 0)
fn approximateGivensQuaternion(A: ptr<function, Symmetric3x3>) -> givens {
    let g = givens(2.f * ((*A).mxx - (*A).myy), (*A).myx);
    var b = GAMMA * g.sh * g.sh < g.ch * g.ch;
    let w = rsqrt(fma(g.ch, g.ch, g.sh * g.sh));
    if w != w {
        b = false;
    }

    if b {
        return givens(w * g.ch, w * g.sh);
    } else {
        return givens(CSTAR, SSTAR);
    }
}

// Function used to apply a givens rotation S. Calculates the weights and updates the quaternion to contain the cumultative rotation
fn jacobiConjugation(x: i32, y: i32, z: i32, S: ptr<function, Symmetric3x3>, q: ptr<function, Quat>) {
    var g = approximateGivensQuaternion(S);
    let scale = 1.f / fma(g.ch, g.ch, g.sh * g.sh);
    let a = fma(g.ch, g.ch, -g.sh * g.sh) * scale;
    let b = 2.f * g.sh * g.ch * scale;
    var _S = (*S);
    // perform conjugation S = Q'*S*Q
    (*S).mxx = fma(a, fma(a, _S.mxx, b * _S.myx), b * (fma(a, _S.myx, b * _S.myy)));
    (*S).myx = fma(a, fma(-b, _S.mxx, a * _S.myx), b * (fma(-b, _S.myx, a * _S.myy)));
    (*S).myy = fma(-b, fma(-b, _S.mxx, a * _S.myx), a * (fma(-b, _S.myx, a * _S.myy)));
    (*S).mzx = fma(a, _S.mzx, b * _S.mzy);
    (*S).mzy = fma(-b, _S.mzx, a * _S.mzy);
    (*S).mzz = _S.mzz;
    // update cumulative rotation qV
    var tmp = array<f32, 3>( // TODO: why does it have to be `var` instead of `let` so we can index with z?
        (*q).coords[0] * g.sh,
        (*q).coords[1] * g.sh,
        (*q).coords[2] * g.sh);
    g.sh *= (*q).coords[3];
    // (x,y,z) corresponds to ((0,1,2),(1,2,0),(2,0,1)) for (p,q) = ((0,1),(1,2),(0,2))
    (*q).coords[z] = fma((*q).coords[z], g.ch, g.sh);
    (*q).coords[3] = fma((*q).coords[3], g.ch, -tmp[z]); // w
    (*q).coords[x] = fma((*q).coords[x], g.ch, tmp[y]);
    (*q).coords[y] = fma((*q).coords[y], g.ch, -tmp[x]);
    // re-arrange matrix for next iteration
    _S.mxx = (*S).myy;
    _S.myx = (*S).mzy; _S.myy = (*S).mzz;
    _S.mzx = (*S).myx; _S.mzy = (*S).mzx; _S.mzz = (*S).mxx;
    (*S).mxx = _S.mxx;
    (*S).myx = _S.myx; (*S).myy = _S.myy;
    (*S).mzx = _S.mzx; (*S).mzy = _S.mzy; (*S).mzz = _S.mzz;
}

// Function used to contain the givens permutations and the loop of the jacobi steps controlled by JACOBI_STEPS
// Returns the quaternion q containing the cumultative result used to reconstruct S
fn jacobiEigenanalysis(S: Symmetric3x3) -> Quat {
    var mat = S;
    var q = identity();
    for (var i = 0u; i < JACOBI_STEPS; i += 1u) {
        jacobiConjugation(0, 1, 2, &mat, &q);
        jacobiConjugation(1, 2, 0, &mat, &q);
        jacobiConjugation(2, 0, 1, &mat, &q);
    }
    return q;
}

struct SortedSingularValues {
    B: mat3x3<f32>,
    V: mat3x3<f32>,
}

/// Implementation of Algorithm 3
// NOTE: doing this through pointers to B and fails with an internal error in naga
//       when trying to swap their columns.
fn sortSingularValues(B: mat3x3<f32>, V: mat3x3<f32>) -> SortedSingularValues {
    var bx = B[0];
    var by = B[1];
    var bz = B[2];
    var vx = V[0];
    var vy = V[1];
    var vz = V[2];
    var rho1 = dot(bx, bx);
    var rho2 = dot(by, by);
    var rho3 = dot(bz, bz);

    var c = rho1 < rho2;
    condNegSwapVec(c, &bx, &by);
    condNegSwapVec(c, &vx, &vy);
    condSwap(c, &rho1, &rho2);
    c = rho1 < rho3;
    condNegSwapVec(c, &bx, &bz);
    condNegSwapVec(c, &vx, &vz);
    condSwap(c, &rho1, &rho3);
    c = rho2 < rho3;
    condNegSwapVec(c, &by, &bz);
    condNegSwapVec(c, &vy, &vz);

    return SortedSingularValues(mat3x3(bx, by, bz), mat3x3(vx, vy, vz));
}

// Implementation of Algorithm 4
fn QRGivensQuaternion(a1: f32, a2: f32) -> givens {
    // a1 = pivot point on diagonal
    // a2 = lower triangular entry we want to annihilate
    let epsilon = SVD_EPSILON;
    let rho = accurateSqrt(fma(a1, a1, a2 * a2));
    var ch = abs(a1) + max(rho, epsilon);
    var sh = select(0.0, a2, rho > epsilon);
    let b = a1 < 0.f;
    condSwap(b, &sh, &ch);
    let w = rsqrt(fma(ch, ch, sh * sh));
    ch *= w;
    sh *= w;
    return givens(ch, sh);
}

// Implements a QR decomposition of a Matrix, see Sec 4.2
fn QRDecomposition(in_B: mat3x3<f32>) -> QR {
    var B = in_B;

    // first givens rotation (ch,0,0,sh)
    let g1 = QRGivensQuaternion(B[0].x, B[0].y);
    var a = fma(-2.f, g1.sh * g1.sh, 1.f);
    var b = 2.f * g1.ch * g1.sh;
    // apply B = Q' * B
    var r00 = fma(a, B[0].x, b * B[0].y);  var r01 = fma(a, B[1].x, b * B[1].y);	var r02 = fma(a, B[2].x, b * B[2].y);
    var r10 = fma(-b, B[0].x, a * B[0].y); var r11 = fma(-b, B[1].x, a * B[1].y);	var r12 = fma(-b, B[2].x, a * B[2].y);
    var r20 = B[0].z;					 var r21 = B[1].z;						var r22 = B[2].z;
    // second givens rotation (ch,0,-sh,0)
    let g2 = QRGivensQuaternion(r00, r20);
    a = fma(-2.f, g2.sh * g2.sh, 1.f);
    b = 2.f * g2.ch * g2.sh;
    // apply B = Q' * B;
    var b00 = fma(a, r00, b * r20);  var b01 = fma(a, r01, b * r21);  var b02 = fma(a, r02, b * r22);
    var b10 = r10;                   var b11 = r11;                   var b12 = r12;
    var b20 = fma(-b, r00, a * r20); var b21 = fma(-b, r01, a * r21); var b22 = fma(-b, r02, a * r22);
    // third givens rotation (ch,sh,0,0)
    let g3 = QRGivensQuaternion(b11, b21);
    a = fma(-2.f, g3.sh * g3.sh, 1.f);
    b = 2.f * g3.ch * g3.sh;
    // R is now set to desired value
    r00 = b00;                   r01 = b01;                  r02 = b02;
    r10 = fma(a, b10, b * b20);  r11 = fma(a, b11, b * b21);  r12 = fma(a, b12, b * b22);
    r20 = fma(-b, b10, a * b20); r21 = fma(-b, b11, a * b21); r22 = fma(-b, b12, a * b22);
    // construct the cumulative rotation Q=Q1 * Q2 * Q3
    // the number of floating point operations for three quaternion multiplications
    // is more or less comparable to the explicit form of the joined matrix.
    // certainly more memory-efficient!
    let sh12 = 2.f * fma(g1.sh, g1.sh, -0.5f);
    let sh22 = 2.f * fma(g2.sh, g2.sh, -0.5f);
    let sh32 = 2.f * fma(g3.sh, g3.sh, -0.5f);

    let q00 = sh12 * sh22;
    let q01 = fma(4.f * g2.ch * g3.ch, sh12 * g2.sh * g3.sh, 2.f * g1.ch * g1.sh * sh32);
    let q02 = fma(4.f * g1.ch * g3.ch, g1.sh * g3.sh, -2.f * g2.ch * sh12 * g2.sh * sh32);

    let q10 = -2.f * g1.ch * g1.sh * sh22;
    let q11 = fma(-8.f * g1.ch * g2.ch * g3.ch, g1.sh * g2.sh * g3.sh, sh12 * sh32);
    let q12 = fma(-2.f * g3.ch, g3.sh, 4.f * g1.sh * fma(g3.ch * g1.sh, g3.sh, g1.ch * g2.ch * g2.sh * sh32));

    let q20 = 2.f * g2.ch * g2.sh;
    let q21 = -2.f * g3.ch * sh22 * g3.sh;
    let q22 = sh22 * sh32;

    let Q = mat3x3(
        vec3(q00, q10, q20),
        vec3(q01, q11, q21),
        vec3(q02, q12, q22),
    );
    let R = mat3x3(
        vec3(r00, r10, r20),
        vec3(r01, r11, r21),
        vec3(r02, r12, r22),
    );

    return QR(Q, R);
}

// Computes the SVD of a 3x3 matrix.
fn svd(A: mat3x3<f32>) -> Svd {
    let ata = transpose(A) * A;
    let ata_sym = Symmetric3x3(
        ata[0].x,
        ata[0].y, ata[1].y,
        ata[0].z, ata[1].z, ata[2].z
    );
    let V = toMatrix(jacobiEigenanalysis(ata_sym));
    let B = A * V;
    let sorted = sortSingularValues(B, V);
    let qr = QRDecomposition(sorted.B);
    let S = vec3(qr.R[0].x, qr.R[1].y, qr.R[2].z);
    return Svd(qr.Q, S, transpose(sorted.V));
}

// Rebuilds the matrix this svd is the decomposition of.
fn recompose(svd: Svd) -> mat3x3<f32> {
    let U_S = mat3x3(svd.U[0] * svd.S.x, svd.U[1] * svd.S.y, svd.U[2] * svd.S.z);
    return U_S * svd.Vt;
}
