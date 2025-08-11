const subtle = crypto.subtle;

function arrayBufferToBase64(buffer: ArrayBuffer): string {
	const bytes = new Uint8Array(buffer);
	let binary = '';
	for (let i = 0; i < bytes.byteLength; i++) {
		binary += String.fromCharCode(bytes[i]);
	}
	return btoa(binary);
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
	const binary = atob(base64);
	const len = binary.length;
	const bytes = new Uint8Array(len);
	for (let i = 0; i < len; i++) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes.buffer;
}

/** Generate a secure random AES-GCM 256-bit key */
export async function generateSharedKey(): Promise<CryptoKey> {
	return subtle.generateKey(
		{
			name: 'AES-GCM',
			length: 256
		},
		true, // extractable (for export/import if needed)
		['encrypt', 'decrypt']
	);
}

/**
 * Encrypt a string with a shared AES-GCM key.
 * Output is Base64(IV + ciphertext).
 */
export async function encryptWithSharedKey(
	key: CryptoKey | string,
	plaintext: string
): Promise<string> {
	if (typeof key === 'string') {
		key = await importSharedKey(key);
	}
	const iv = crypto.getRandomValues(new Uint8Array(12)); // 96-bit nonce
	const encoded = new TextEncoder().encode(plaintext);
	const ciphertext = await subtle.encrypt(
		{
			name: 'AES-GCM',
			iv
		},
		key,
		encoded
	);

	// concat IV + ciphertext
	const combined = new Uint8Array(iv.length + ciphertext.byteLength);
	combined.set(iv, 0);
	combined.set(new Uint8Array(ciphertext), iv.length);

	return arrayBufferToBase64(combined.buffer);
}

/**
 * Decrypt from Base64(IV + ciphertext) using shared AES-GCM key.
 */
export async function decryptWithSharedKey(
	key: CryptoKey | string,
	dataB64: string
): Promise<string> {
	if (typeof key === 'string') {
		key = await importSharedKey(key);
	}
	const combined = new Uint8Array(base64ToArrayBuffer(dataB64));
	const iv = combined.slice(0, 12);
	const ciphertext = combined.slice(12);

	const decrypted = await subtle.decrypt(
		{
			name: 'AES-GCM',
			iv
		},
		key,
		ciphertext
	);

	return new TextDecoder().decode(decrypted);
}

/** Export shared AES key as Base64 raw */
export async function exportSharedKey(key?: CryptoKey): Promise<string> {
	if (!key) key = await generateSharedKey();
	const raw = await subtle.exportKey('raw', key);
	return arrayBufferToBase64(raw);
}

/** Import shared AES key from Base64 raw */
export async function importSharedKey(rawB64: string): Promise<CryptoKey> {
	const raw = base64ToArrayBuffer(rawB64);
	return subtle.importKey('raw', raw, 'AES-GCM', true, ['encrypt', 'decrypt']);
}
