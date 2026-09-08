/**
 * Rotation helpers for the assembly panel: unit quaternions `[x, y, z, w]`
 * (the wire/three.js order) ↔ Euler angles in DEGREES in three.js's default
 * intrinsic `XYZ` order — the same convention `THREE.Euler.setFromQuaternion`
 * uses in the viewport, so what the panel shows is what the viewport applies.
 */

const DEG = Math.PI / 180;

/** @param {[number, number, number]} deg */
export function eulerDegToQuat([x, y, z]) {
	// three.js Quaternion.setFromEuler, order XYZ.
	const c1 = Math.cos((x * DEG) / 2), c2 = Math.cos((y * DEG) / 2), c3 = Math.cos((z * DEG) / 2);
	const s1 = Math.sin((x * DEG) / 2), s2 = Math.sin((y * DEG) / 2), s3 = Math.sin((z * DEG) / 2);
	const q = [
		s1 * c2 * c3 + c1 * s2 * s3,
		c1 * s2 * c3 - s1 * c2 * s3,
		c1 * c2 * s3 + s1 * s2 * c3,
		c1 * c2 * c3 - s1 * s2 * s3
	];
	const n = Math.hypot(q[0], q[1], q[2], q[3]) || 1;
	return q.map((v) => v / n);
}

/** @param {[number, number, number, number]} q */
export function quatToEulerDeg(q) {
	const [x, y, z, w] = q;
	// Rotation matrix elements (row-major m_rc), three.js makeRotationFromQuaternion.
	const x2 = x + x, y2 = y + y, z2 = z + z;
	const xx = x * x2, xy = x * y2, xz = x * z2;
	const yy = y * y2, yz = y * z2, zz = z * z2;
	const wx = w * x2, wy = w * y2, wz = w * z2;
	const m11 = 1 - (yy + zz), m12 = xy - wz, m13 = xz + wy;
	const m22 = 1 - (xx + zz), m23 = yz - wx;
	const m32 = yz + wx, m33 = 1 - (xx + yy);
	// Euler.setFromRotationMatrix, order XYZ.
	const ey = Math.asin(Math.max(-1, Math.min(1, m13)));
	let ex, ez;
	if (Math.abs(m13) < 0.9999999) {
		ex = Math.atan2(-m23, m33);
		ez = Math.atan2(-m12, m11);
	} else {
		ex = Math.atan2(m32, m22);
		ez = 0;
	}
	const r = (v) => Math.round((v / DEG) * 1000) / 1000;
	return [r(ex), r(ey), r(ez)];
}
