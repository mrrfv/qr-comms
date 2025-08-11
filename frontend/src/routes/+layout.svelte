<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="text-center">
	<h1>QR Comms</h1>
	<p>Quickly and safely share a link or message between your PC and phone using just a QR code.</p>
	<hr />
	{@render children?.()}
	<hr />
	<p>
		<small>Privacy: Data is end-to-end encrypted with a shared key embedded in the QR code.</small>
	</p>
</div>
