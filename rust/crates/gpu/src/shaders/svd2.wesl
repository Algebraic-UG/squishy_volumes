import package::util::sign;

fn identity3() -> mat3x3f {
    return mat3x3f(1., 0., 0., 0., 1., 0., 0., 0., 1.);
}
fn identity2() -> mat2x2f {
    return mat2x2f(1., 0., 0., 1.);
}

fn outer3(a: vec3f, b: vec3f) -> mat3x3f {
    return mat3x3f(
        a * b.x,
        a * b.y,
        a * b.z,
    );
}

fn outer2(a: vec2f, b: vec2f) -> mat2x2f {
    return mat2x2f(
        a * b.x,
        a * b.y,
    );
}

fn from_diagonal(diagonal: vec3f) -> mat3x3f {
    return mat3x3f(
        diagonal.x, 0., 0.,
        0., diagonal.y, 0.,
        0., 0., diagonal.z,
    );
}

struct Qr {
    q: mat3x3<f32>,
    r: mat3x3<f32>,
};

fn qr(m: mat3x3<f32>) -> Qr {
    var qr: Qr;
    qr.r = m;

    {
        let x = qr.r[0];
        let a = -sign(x.x) * length(x);
        let v = normalize(x - a * vec3f(1., 0., 0.));
        qr.q = identity3() - 2. * outer3(v, v);
    }

    qr.r = qr.q * qr.r;

    {
        let x = m[1].xy;
        let a = -sign(x.x) * length(x);
        let v = normalize(x - a * vec2f(1., 0.));
        let p2 = identity2() - 2. * outer2(v, v);

        let p = mat3x3f(
            vec3f(1., 0., 0.),
            vec3f(0., p2[0]),
            vec3f(0., p2[1]),
        );

        qr.r = p * qr.r;
        qr.q = qr.q * p;
    }

    return qr;
}

struct SymmetricEigen {
    v: mat3x3f,
    d: vec3f,
}

fn symmetric_eigen(m_in: mat3x3f, n: u32) -> SymmetricEigen {
    var m = m_in;

    var e: SymmetricEigen;
    e.v = identity3();
    for (var i = 0u; i < n; i++) {
        let qr = qr(m);
        m = qr.r * qr.q;
        e.v *= qr.q;
    }
    e.d = vec3f(m[0].x, m[1].y, m[2].z);

    return e;
}

struct Svd {
    u: mat3x3<f32>,
    s: vec3<f32>,
    v: mat3x3<f32>,
};

fn svd(m: mat3x3<f32>, n: u32) -> Svd {
    var svd: Svd;
    let symmetric_eigen = symmetric_eigen(transpose(m) * m, n);
    svd.v = symmetric_eigen.v;
    svd.s = sqrt(symmetric_eigen.d);

    svd.u = m * svd.v * from_diagonal(1. / svd.s);

    return svd;
}

fn recompose(svd: Svd) -> mat3x3<f32> {
    return svd.u * from_diagonal(svd.s) * transpose(svd.v);
}
