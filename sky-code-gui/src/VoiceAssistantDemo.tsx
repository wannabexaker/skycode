/**
 * SkyCode Voice Assistant Demo
 * 
 * Standalone demo showing all voice chat capabilities.
 * Includes speech-to-text, text-to-speech, and visual feedback.
 */

import { useState } from "react";
import { Sun, Moon, Mic } from "lucide-react";
import { Button } from "./components/ui/button";
import { VoiceChat } from "./components/ui/voice-chat";

export default function VoiceAssistantDemo() {
  const [isDark, setIsDark] = useState(true);
  const [transcript, setTranscript] = useState("");
  const [messages, setMessages] = useState<string[]>([]);

  const handleTranscript = (text: string) => {
    console.log("Transcript:", text);
    setTranscript(text);
    setMessages((prev) => [...prev, `You said: ${text}`]);
    
    // Simulate AI response
    setTimeout(() => {
      const response = `I heard you say: "${text}". This is a demo response.`;
      setMessages((prev) => [...prev, `AI: ${response}`]);
      
      // Speak the response if speech synthesis is available
      if (window.speechSynthesis) {
        const utterance = new SpeechSynthesisUtterance(response);
        window.speechSynthesis.speak(utterance);
      }
    }, 1000);
  };

  return (
    <div className={isDark ? "dark" : ""}>
      <div className="min-h-screen bg-bg-0 text-text-100 transition-colors duration-200">
        {/* Header */}
        <div className="border-b border-bg-300 px-6 py-4">
          <div className="mx-auto flex max-w-4xl items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold font-mono text-accent">
                ϟ SkyCode Voice Demo
              </h1>
              <p className="text-sm text-text-400 mt-1">
                Speech-to-Text + Text-to-Speech powered by Web Speech API
              </p>
            </div>
            <Button
              variant="outline"
              size="icon"
              onClick={() => setIsDark(!isDark)}
              aria-label="Toggle theme"
            >
              {isDark ? <Sun size={16} /> : <Moon size={16} />}
            </Button>
          </div>
        </div>

        {/* Content */}
        <div className="mx-auto max-w-4xl px-6 py-8">
          
          {/* Voice Chat Component */}
          <div className="mb-8">
            <VoiceChat
              onTranscript={handleTranscript}
              onStart={() => console.log("Voice recording started")}
              onStop={(duration) => console.log(`Voice recording stopped after ${duration}s`)}
              onVolumeChange={(volume) => console.log(`Volume: ${volume}%`)}
              onSpeechStart={() => console.log("AI started speaking")}
              onSpeechEnd={() => console.log("AI finished speaking")}
              useWebAPI={true}
            />
          </div>

          {/* Instructions */}
          <div className="mb-8 rounded-xl border border-bg-300 bg-bg-100 p-6">
            <h2 className="text-lg font-semibold mb-4 font-mono text-text-200">
              How to Use
            </h2>
            <ul className="space-y-2 text-sm text-text-300">
              <li className="flex items-start gap-2">
                <span className="text-accent font-bold">1.</span>
                <span>Click the large microphone button to start voice recording</span>
              </li>
              <li className="flex items-start gap-2">
                <span className="text-accent font-bold">2.</span>
                <span>Speak clearly into your microphone</span>
              </li>
              <li className="flex items-start gap-2">
                <span className="text-accent font-bold">3.</span>
                <span>Watch the waveform visualization respond to your voice</span>
              </li>
              <li className="flex items-start gap-2">
                <span className="text-accent font-bold">4.</span>
                <span>Click again to stop recording and send the transcript</span>
              </li>
              <li className="flex items-start gap-2">
                <span className="text-accent font-bold">5.</span>
                <span>AI will respond with speech synthesis automatically</span>
              </li>
            </ul>
          </div>

          {/* Current Transcript */}
          {transcript && (
            <div className="mb-8 rounded-xl border border-accent/20 bg-accent/5 p-6">
              <h3 className="text-sm font-semibold mb-2 text-accent font-mono uppercase tracking-wider">
                Latest Transcript
              </h3>
              <p className="text-text-200">{transcript}</p>
            </div>
          )}

          {/* Message History */}
          {messages.length > 0 && (
            <div className="rounded-xl border border-bg-300 bg-bg-100 p-6">
              <h3 className="text-sm font-semibold mb-4 font-mono text-text-200 uppercase tracking-wider">
                Conversation History
              </h3>
              <div className="space-y-3">
                {messages.map((msg, idx) => (
                  <div
                    key={idx}
                    className={`p-3 rounded-lg text-sm ${
                      msg.startsWith("You")
                        ? "bg-accent/10 text-text-200 border border-accent/20"
                        : "bg-bg-200 text-text-300 border border-bg-300"
                    }`}
                  >
                    {msg}
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Browser Compatibility Notice */}
          <div className="mt-8 rounded-xl border border-yellow-500/20 bg-yellow-500/5 p-4">
            <div className="flex items-start gap-3">
              <span className="text-2xl">⚠️</span>
              <div className="text-xs text-text-400">
                <p className="font-semibold mb-1 text-text-300">Browser Compatibility:</p>
                <p>
                  Speech-to-Text works best in <strong>Chrome, Edge, and Safari</strong>.
                  Firefox has limited support. Make sure to allow microphone access when prompted.
                </p>
              </div>
            </div>
          </div>

          {/* Features Grid */}
          <div className="mt-8 grid gap-4 sm:grid-cols-2">
            <div className="rounded-xl border border-bg-300 bg-bg-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <Mic className="w-4 h-4 text-accent" />
                <h4 className="font-semibold text-sm text-text-200">Speech-to-Text</h4>
              </div>
              <p className="text-xs text-text-400">
                Real-time voice recognition using Web Speech API with continuous listening support.
              </p>
            </div>

            <div className="rounded-xl border border-bg-300 bg-bg-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <svg className="w-4 h-4 text-accent" fill="currentColor" viewBox="0 0 20 20">
                  <path d="M10 12a2 2 0 100-4 2 2 0 000 4z" />
                  <path fillRule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clipRule="evenodd" />
                </svg>
                <h4 className="font-semibold text-sm text-text-200">Visual Feedback</h4>
              </div>
              <p className="text-xs text-text-400">
                Real-time waveform visualization, volume meter, and animated particles for ambient effect.
              </p>
            </div>

            <div className="rounded-xl border border-bg-300 bg-bg-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <svg className="w-4 h-4 text-accent" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clipRule="evenodd" />
                </svg>
                <h4 className="font-semibold text-sm text-text-200">Text-to-Speech</h4>
              </div>
              <p className="text-xs text-text-400">
                Native browser TTS for AI responses with customizable voice, rate, and pitch settings.
              </p>
            </div>

            <div className="rounded-xl border border-bg-300 bg-bg-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <svg className="w-4 h-4 text-accent" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm1-12a1 1 0 10-2 0v4a1 1 0 00.293.707l2.828 2.829a1 1 0 101.415-1.415L11 9.586V6z" clipRule="evenodd" />
                </svg>
                <h4 className="font-semibold text-sm text-text-200">Smart Timing</h4>
              </div>
              <p className="text-xs text-text-400">
                Duration tracking, auto-stop on silence, and intelligent state management for seamless UX.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
