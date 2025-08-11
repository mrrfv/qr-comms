<script>
	import { exportSharedKey } from '$lib/crypto';
	import { onMount } from 'svelte';

	// @ts-expect-error There are no type declarations for svelte-qrcode
	import QrCode from 'svelte-qrcode';

	// Get base URL
	let baseUrl =
		typeof window !== 'undefined'
			? `${window.location.protocol}//${window.location.hostname}${window.location.port ? ':' + window.location.port : ''}/`
			: '';

	let qr_code_location = '';
	onMount(async () => {
		// Generate a short-lived channel ID and shared encryption key
		const channel_id = crypto.randomUUID();
		const key = await exportSharedKey();
		qr_code_location = baseUrl + 'channel#' + channel_id + '|' + key;
	});
</script>

<p>To connect to this device, scan the QR code below.</p>
<div class="qrcode">
	<QrCode errorCorrection="L" size="250" value={qr_code_location}></QrCode>
</div>
<p>When connected, click <a href={qr_code_location}>here</a> to start!</p>

<style>
	.qrcode {
		margin: 20px 0;
		padding: 30px;
		border: 1px solid #ccc;
		border-radius: 8px;
		background-color: #f9f9f9;
		text-align: center;
		display: flex;
		justify-content: center;
		align-items: center;
	}
</style>
