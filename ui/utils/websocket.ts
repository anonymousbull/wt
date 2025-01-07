class WebSocketManager {
	private socket: WebSocket | null = null;
	private messageHandler: ((message: string) => void) | null = null;

	connect(url: string) {
		this.socket = new WebSocket(url);

		this.socket.onopen = () => {
			console.log('WebSocket connection established');
		};

		this.socket.onmessage = (event) => {
			const message = event.data;
			if (this.messageHandler) {
				this.messageHandler(message);
			}
		};

		this.socket.onerror = (error) => {
			console.error('WebSocket error:', error);
		};

		this.socket.onclose = () => {
			console.log('WebSocket connection closed');
		};
	}

	disconnect() {
		if (this.socket) {
			this.socket.close();
		}
	}

	setMessageHandler(handler: (message: string) => void) {
		this.messageHandler = handler;
	}
}

export const webSocketManager = new WebSocketManager();

