<script lang="ts">
	import { goto } from '$app/navigation';
	import { decryptWithSharedKey, encryptWithSharedKey, exportSharedKey } from '$lib/crypto';
	import { PUBLIC_WS_BASE_URL } from '$env/static/public';
	import { onMount } from 'svelte';

	interface Message {
		id: string;
		content: string;
		timestamp: Date;
		owned: boolean;
	}

	let status = '';
	let message = '';
	let messages: Message[] = [];
	let messagesEndEl: Element;
	let socket: WebSocket | null = null;

	// Get channel ID
	const url_data = window.location.hash.replace('#', '').split('|');
	window.location.hash = '';
	const channel_id = url_data[0];
	let shared_key = url_data[1];

	if (!channel_id) goto('/');

	function connectWebSocket() {
		socket = new WebSocket(`${PUBLIC_WS_BASE_URL}/${channel_id}`);
		status = `Connecting to ${channel_id}...`;

		socket?.addEventListener('error', () => {
			status = `Connection to ${channel_id} failed, sorry.`;
		});

		// Connection opened
		socket?.addEventListener('open', () => {
			status = `Connected to ${channel_id}`;
		});

		socket?.addEventListener('close', () => {
			status = `Connection to server closed. Try reconnecting.`;
		});

		// Listen for messages
		socket?.addEventListener('message', async (event) => {
			const decrypted_message_raw = await decryptWithSharedKey(shared_key, event.data).catch(
				(e) => {
					alert(
						`Failed to decrypt message. If this persists, restart your communication with another QR code.`
					);
					console.error(e);
				}
			);
			const decrypted_message = JSON.parse(decrypted_message_raw);
			shared_key = decrypted_message.new_shared_key;
			add_new_message(decrypted_message.message, false);
		});
	}

	function reconnect() {
		if (socket) {
			socket.close();
		}
		connectWebSocket();
	}

	async function send_message() {
		if (!socket || !(socket.readyState === socket.OPEN)) return alert('Not connected to server.');
		// Generate a new shared key for basic forward secrecy
		const new_shared_key = await exportSharedKey();
		const encrypted_message = await encryptWithSharedKey(
			shared_key,
			JSON.stringify({
				message,
				new_shared_key
			})
		);
		socket?.send(encrypted_message);
		add_new_message(message, true);
		shared_key = new_shared_key;
		message = '';
	}

	function add_new_message(content: string, own: boolean) {
		const new_msg: Message = {
			id: crypto.randomUUID(),
			content,
			timestamp: new Date(),
			owned: own
		};
		messages = [...messages, new_msg];
	}

	$: {
		if (messagesEndEl) {
			messagesEndEl.scrollIntoView({ behavior: 'smooth' });
		}
	}

	// Escape HTML to prevent injection
	function escapeHTML(str: string) {
		return str.replace(
			/[&<>"']/g,
			(match) =>
				({
					'&': '&amp;',
					'<': '&lt;',
					'>': '&gt;',
					'"': '&quot;',
					"'": '&#39;'
				})[match]
		);
	}

	// Replace URLs with clickable links
	function linkify(text: string) {
		const escaped = escapeHTML(text);
		const urlRegex = /(https?:\/\/[^\s]+|www\.[^\s]+)/g;
		return escaped.replace(urlRegex, (rawUrl) => {
			const href = rawUrl.startsWith('www.') ? `https://${rawUrl}` : rawUrl;
			return `<a href="${href}" target="_blank" rel="noopener noreferrer">${rawUrl}</a>`;
		});
	}

	onMount(async () => {
		// Connect to server
		connectWebSocket();
	});
</script>

<svelte:head>
	<title>QR Comms</title>
</svelte:head>

<p>{status}</p>
<p><small>Do not send messages until both devices are connected.</small></p>
{#if status && (!socket || !(socket.readyState == socket.OPEN))}
	<button onclick={reconnect}>Reconnect</button>
{/if}

<div class="chat-container">
	<div class="messages">
		{#each messages as msg (msg.id)}
			<div class="message" class:sent={msg.owned}>
				{@html linkify(msg.content)}
			</div>
		{/each}
		<div bind:this={messagesEndEl}></div>
	</div>

	<form onsubmit={send_message}>
		<input bind:value={message} type="text" placeholder="Message..." />
		<button type="submit">Send</button>
	</form>
</div>

<style>
	.chat-container {
		display: flex;
		flex-direction: column;
		max-width: 500px;
		margin: auto;
		padding: 1rem;
		font-family: system-ui, sans-serif;
		background: #f5f5f5;
		border-radius: 1rem;
		box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
	}

	.messages {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		margin-bottom: 0.75rem;
		overflow-y: auto;
		max-height: 400px;
	}

	.message {
		font-size: 1rem;
		background: white;
		color: black;
		padding: 0.75rem 1rem;
		border-radius: 1rem;
		margin: 0.25rem 0;
		max-width: 80%;
		box-shadow: 0 2px 6px rgba(0, 0, 0, 0.05);
		word-wrap: break-word;
		animation: fadeIn 0.3s ease;
	}

	.message.sent {
		align-self: flex-end;
		background: #007bff;
		color: white;
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
			transform: translateY(5px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	form {
		display: flex;
		gap: 0.5rem;
	}

	input {
		flex: 1;
		padding: 0.5rem 0.75rem;
		border: 1px solid #ccc;
		border-radius: 0.75rem;
		font-size: 1rem;
	}

	button {
		background: #007bff;
		color: white;
		border: none;
		padding: 0.5rem 1rem;
		border-radius: 0.75rem;
		cursor: pointer;
	}

	button:hover {
		background: #0056b3;
	}
</style>
