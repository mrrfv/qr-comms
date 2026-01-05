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
					return ''; // Return empty string to handle the error case
				}
			);
			if (decrypted_message_raw) {
				const decrypted_message = JSON.parse(decrypted_message_raw);
				shared_key = decrypted_message.new_shared_key;
				add_new_message(decrypted_message.message, false);
			}
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
			(match: string) =>
				({
					'&': '&',
					'<': '<',
					'>': '>',
					'"': '"',
					"'": '&#39;'
				}[match] || match)
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

	// Copy text to clipboard
	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text)
		.then(() => {
			alert('Copied to clipboard!');
		})
		.catch((err) => {
			console.error('Could not copy text: ', err);
			alert('Failed to copy to clipboard.');
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

{#if status && (!socket || !(socket.readyState == socket.OPEN))}
	<button onclick={reconnect}>Reconnect</button>
{/if}

<div class="chat-container">
	<div class="messages">
		{#each messages as msg (msg.id)}
			<button
				class="message"
				onclick={() => copyToClipboard(msg.content.trim())}
				onkeydown={(e) => e.key === 'Enter' && copyToClipboard(msg.content.trim())}
				class:sent={msg.owned}
				aria-label="Copy message to clipboard"
				tabindex="0">
				{@html linkify(msg.content)}
			</button>
		{/each}
		<div bind:this={messagesEndEl}></div>
	</div>

	<p class="fineprint" class:hidden={messages.length}>Do not send messages until both devices are connected.</p>
	<form onsubmit={send_message}>
		<input bind:value={message} type="text" placeholder="Message..." />
		<button type="submit">Send</button>
	</form>
</div>

<style>
	.chat-container {
		display: flex;
		flex-direction: column;
		max-width: 80%;
		height: 400px;
		margin: auto;
		padding: 1rem;
		font-family: system-ui, sans-serif;
		background: #f5f5f5;
		border-radius: 1rem;
		box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
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
		transition: all 0.2s;
		text-align: left;
	}

	.message::after {
		content: '';
		display: block;
		font-size: 0.7rem;
		color: #666;
		opacity: 0; /* Start invisible */
		transition: opacity 0.3s ease; /* Smooth transition for opacity */
	}

	.message:hover::after {
		content: ' (click to copy)';
		opacity: 1; /* Fade in on hover */
	}

	.message.sent {
		align-self: flex-end;
		background: #007bff;
		color: white;
		text-align: right;
	}

	.message.sent::after {
		color: #cce5ff;
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

	.fineprint {
		font-size: 0.9rem;
		color: #666;
		margin-bottom: 0.5rem;
		text-align: center;
	}

	.hidden {
		display: none;
	}
</style>
