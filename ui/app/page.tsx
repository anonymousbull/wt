"use client"

import React, { useState, useRef, useEffect } from 'react'
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Button } from "@/components/ui/button"
import { generateTradingLogs } from '../utils/tradingLogGenerator'
import {webSocketManager} from "@/utils/websocket";


type Log = {
	timestamp: string
	content: string
	isUserInput?: boolean
}

const TradingTerminal: React.FC = () => {
	const [logs, setLogs] = useState<Log[]>([])
	const [filter, setFilter] = useState('')
	const [isConnected, setIsConnected] = useState(false)
	const [isStreaming, setIsStreaming] = useState(false)
	const [userInput, setUserInput] = useState('')
	const scrollAreaRef = useRef<HTMLDivElement>(null)

	useEffect(() => {
		const wsUrl = 'wss://localhost:8080' // Replace with your actual WebSocket URL
		webSocketManager.setMessageHandler((message) => {
			const newLog = {
				timestamp: new Date().toISOString(),
				content:message
			}
			setLogs(prevLogs => [...prevLogs, newLog])
		})

		return () => {
			webSocketManager.disconnect()
		}
	}, [])

	useEffect(() => {
		const initialLogs = generateTradingLogs(50).map(log => ({
			timestamp: new Date().toISOString(),
			content: log
		}))
		setLogs(initialLogs)
	}, [])


	useEffect(() => {
		let interval: NodeJS.Timeout | null = null
		if (isStreaming) {
			interval = setInterval(() => {
				const newLog = generateTradingLogs(1)[0]
				setLogs(prevLogs => [...prevLogs, { timestamp: new Date().toISOString(), content: newLog }])
			}, 1000)
		}
		return () => {
			if (interval) clearInterval(interval)
		}
	}, [isStreaming])

	useEffect(() => {
		if (scrollAreaRef.current) {
			scrollAreaRef.current.scrollTop = scrollAreaRef.current.scrollHeight
		}
	}, [logs])

	const filteredLogs = logs.filter(log =>
		log.content.toLowerCase().includes(filter.toLowerCase())
	)

	const colorizeLog = (log: string, isUserInput: boolean = false) => {
		return log.split(' ').map((part, index) => {
			const [key, value] = part.split('=')
			let valueColor = isUserInput ? 'text-[#ffd866]' : 'text-[#a9dc76]'  // Default color

			if (!isUserInput) {
				switch (key) {
					case 'id':
						valueColor = 'text-[#ab9df2]'
						break
					case 'state':
						valueColor = value.includes('Success') ? 'text-[#a9dc76]' :
							value.includes('Failed') ? 'text-[#ff6188]' :
								value.includes('Pending') ? 'text-[#ffd866]' : 'text-[#78dce8]'
						break
					case 'signature':
					case 'amm':
						valueColor = 'text-[#78dce8]'
						return (
							<span key={index}>
                <span className="text-[#78dce8]">{key}</span>
                <span className="text-[#fcfcfa]">=</span>
                <a href={value} target="_blank" rel="noopener noreferrer" className={`${valueColor} hover:underline`}>
                  {value.substring(2, 15).split('/').pop()}
                </a>
								{' '}
              </span>
						)
					case 'pct':
						valueColor = parseFloat(value) >= 0 ? 'text-[#a9dc76]' : 'text-[#ff6188]'
						break
				}
			}

			return (
				<span key={index}>
          <span className="text-[#78dce8]">{key}</span>
          <span className="text-[#fcfcfa]">=</span>
          <span className={valueColor}>{value} </span>
        </span>
			)
		})
	}

	const handleUserInput = (e: React.FormEvent) => {
		e.preventDefault()
		if (userInput.trim()) {
			const newLog = {
				timestamp: new Date().toISOString(),
				content: userInput,
				isUserInput: true
			}
			setLogs(prevLogs => [...prevLogs, newLog])
			setUserInput('')
		}
	}

	const toggleConnection = () => {
		if (isConnected) {
			webSocketManager.disconnect()
			setIsConnected(false)
		} else {
			webSocketManager.connect('ws://localhost:8080') // Replace with your actual WebSocket URL
			setIsConnected(true)
		}
	}

	return (
		<div className="w-full max-w-4xl mx-auto h-[90vh] bg-[#2d2a2e] rounded-lg overflow-hidden shadow-xl border border-[#727072]">
			<div className="bg-[#403e41] p-2 flex items-center justify-between">
				<div className="flex items-center space-x-2">
					<div className="w-3 h-3 rounded-full bg-[#ff6188]"></div>
					<div className="w-3 h-3 rounded-full bg-[#fc9867]"></div>
					<div className="w-3 h-3 rounded-full bg-[#ffd866]"></div>
					<span className="text-[#fcfcfa] ml-2 text-sm">Trading Engine Terminal</span>
				</div>
				<div className="flex items-center space-x-2">
					<Input
						value={filter}
						onChange={(e) => setFilter(e.target.value)}
						className="w-48 bg-[#2d2a2e] border-none text-[#fcfcfa] focus:outline-none focus:ring-0"
						placeholder="Filter logs..."
					/>
					<Button
						onClick={toggleConnection}
						className={`${isStreaming ? 'bg-[#ff6188]' : 'bg-[#a9dc76]'} text-[#2d2a2e] hover:bg-opacity-80`}
					>
						{isStreaming ? 'Stop' : 'Start'} Stream
					</Button>
				</div>
			</div>
			<ScrollArea className="h-[calc(90vh-112px)] p-4" ref={scrollAreaRef}>
				{filteredLogs.map((log, index) => (
					<div key={index} className="mb-1 font-mono text-sm">
						<span className="text-[#fc9867] mr-2">{new Date(log.timestamp).toLocaleTimeString()}</span>
						{log.isUserInput && <span className="text-[#ffd866] mr-2">[User Input]</span>}
						{colorizeLog(log.content, log.isUserInput)}
					</div>
				))}
			</ScrollArea>
			<form onSubmit={handleUserInput} className="flex items-center p-2 bg-[#403e41] h-14">
				<Input
					value={userInput}
					onChange={(e) => setUserInput(e.target.value)}
					className="flex-grow bg-[#2d2a2e] border-none text-[#fcfcfa] focus:outline-none focus:ring-0 mr-2"
					placeholder="Enter custom log (e.g., id=123 state=BuySuccess signature=https://solscan.io/tx/abc amm=https://solscan.io/account/xyz pct=5.25)"
				/>
				<Button type="submit" className="bg-[#78dce8] text-[#2d2a2e] hover:bg-opacity-80">
					Add Log
				</Button>
			</form>
		</div>
	)
}

export default TradingTerminal

