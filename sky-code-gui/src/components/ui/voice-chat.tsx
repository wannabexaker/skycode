"use client";

import { Mic, Volume2, VolumeX, Zap } from "lucide-react";
import { useState, useEffect, useRef, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "@/lib/utils";

interface VoiceChatProps {
  onTranscript?: (text: string) => void;
  onStart?: () => void;
  onStop?: (duration: number) => void;
  onVolumeChange?: (volume: number) => void;
  onSpeechStart?: () => void;
  onSpeechEnd?: () => void;
  className?: string;
  useWebAPI?: boolean; // Use real Web Speech API
}

interface Particle {
  id: number;
  x: number;
  y: number;
  size: number;
  opacity: number;
  velocity: { x: number; y: number };
}

export function VoiceChat({
  onTranscript,
  onStart,
  onStop,
  onVolumeChange,
  onSpeechStart: _onSpeechStart,
  onSpeechEnd: _onSpeechEnd,
  className,
  useWebAPI = false,
}: VoiceChatProps) {
  const [isListening, setIsListening] = useState(false);
  const [isSpeaking, _setIsSpeaking] = useState(false);
  const [volume, setVolume] = useState(0);
  const [duration, setDuration] = useState(0);
  const [particles, setParticles] = useState<Particle[]>([]);
  const [waveformData, setWaveformData] = useState<number[]>(Array(32).fill(0));
  const [transcript, setTranscript] = useState("");
  
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const animationRef = useRef<number>();
  const recognitionRef = useRef<any>(null);
  const synthRef = useRef<SpeechSynthesis | null>(null);

  // Initialize Web Speech API if available
  useEffect(() => {
    if (typeof window === 'undefined' || !useWebAPI) return;

    // Speech Recognition (Speech-to-Text)
    const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (SpeechRecognition) {
      const recognition = new SpeechRecognition();
      recognition.continuous = true;
      recognition.interimResults = true;
      recognition.lang = 'en-US';

      recognition.onresult = (event: any) => {
        let interimTranscript = '';
        let finalTranscript = '';

        for (let i = event.resultIndex; i < event.results.length; i++) {
          const transcriptPiece = event.results[i][0].transcript;
          if (event.results[i].isFinal) {
            finalTranscript += transcriptPiece + ' ';
          } else {
            interimTranscript += transcriptPiece;
          }
        }

        const fullTranscript = finalTranscript || interimTranscript;
        setTranscript(fullTranscript);
        if (finalTranscript) {
          onTranscript?.(finalTranscript.trim());
        }
      };

      recognition.onerror = (event: any) => {
        console.error('Speech recognition error:', event.error);
        setIsListening(false);
      };

      recognition.onend = () => {
        if (isListening) {
          recognition.start(); // Restart if still supposed to be listening
        }
      };

      recognitionRef.current = recognition;
    }

    // Speech Synthesis (Text-to-Speech)
    if (window.speechSynthesis) {
      synthRef.current = window.speechSynthesis;
    }

    return () => {
      if (recognitionRef.current) {
        try {
          recognitionRef.current.stop();
        } catch (e) {
          // Ignore errors
        }
      }
      if (synthRef.current) {
        synthRef.current.cancel();
      }
    };
  }, [useWebAPI, isListening, onTranscript]);

  // Generate particles for ambient effect
  useEffect(() => {
    const generateParticles = () => {
      const newParticles: Particle[] = [];
      for (let i = 0; i < 20; i++) {
        newParticles.push({
          id: i,
          x: Math.random() * 400,
          y: Math.random() * 400,
          size: Math.random() * 3 + 1,
          opacity: Math.random() * 0.3 + 0.1,
          velocity: {
            x: (Math.random() - 0.5) * 0.5,
            y: (Math.random() - 0.5) * 0.5
          }
        });
      }
      setParticles(newParticles);
    };

    generateParticles();
  }, []);

  // Animate particles
  useEffect(() => {
    const animateParticles = () => {
      setParticles(prev => prev.map(particle => ({
        ...particle,
        x: (particle.x + particle.velocity.x + 400) % 400,
        y: (particle.y + particle.velocity.y + 400) % 400,
        opacity: Math.max(0.1, Math.min(0.4, particle.opacity + (Math.random() - 0.5) * 0.02))
      })));
      animationRef.current = requestAnimationFrame(animateParticles);
    };

    animationRef.current = requestAnimationFrame(animateParticles);
    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, []);

  // Timer and waveform simulation
  useEffect(() => {
    if (isListening) {
      intervalRef.current = setInterval(() => {
        setDuration(prev => prev + 1);
        
        // Simulate audio waveform
        const newWaveform = Array(32).fill(0).map(() => 
          Math.random() * (isListening ? 100 : 20)
        );
        setWaveformData(newWaveform);
        
        // Simulate volume changes
        const newVolume = Math.random() * 100;
        setVolume(newVolume);
        onVolumeChange?.(newVolume);
      }, 100);
    } else {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      setWaveformData(Array(32).fill(0));
      setVolume(0);
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [isListening, onVolumeChange]);

  const handleToggleListening = useCallback(() => {
    if (isListening) {
      // Stop listening
      setIsListening(false);
      onStop?.(duration);
      setDuration(0);
      
      if (useWebAPI && recognitionRef.current) {
        try {
          recognitionRef.current.stop();
        } catch (e) {
          console.error('Error stopping recognition:', e);
        }
      }
    } else {
      // Start listening
      setIsListening(true);
      setTranscript("");
      onStart?.();
      
      if (useWebAPI && recognitionRef.current) {
        try {
          recognitionRef.current.start();
        } catch (e) {
          console.error('Error starting recognition:', e);
        }
      }
    }
  }, [isListening, duration, onStart, onStop, useWebAPI]);

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };

  const getStatusText = () => {
    if (isListening) return "Listening...";
    if (isSpeaking) return "Speaking...";
    return "Tap to speak";
  };

  const getStatusColor = () => {
    if (isListening) return "text-accent";
    if (isSpeaking) return "text-green-400";
    return "text-text-400";
  };

  return (
    <div className={cn("flex flex-col items-center justify-center min-h-[400px] bg-bg-0 relative overflow-hidden rounded-2xl", className)}>
      {/* Ambient particles */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        {particles.map(particle => (
          <motion.div
            key={particle.id}
            className="absolute w-1 h-1 bg-accent/20 rounded-full"
            style={{
              left: particle.x,
              top: particle.y,
              opacity: particle.opacity
            }}
            animate={{
              scale: [1, 1.5, 1],
            }}
            transition={{
              duration: 2,
              repeat: Infinity,
              ease: "easeInOut"
            }}
          />
        ))}
      </div>

      {/* Background glow effects */}
      <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
        <motion.div
          className="w-96 h-96 rounded-full bg-accent/5 blur-3xl"
          animate={{
            scale: isListening ? [1, 1.2, 1] : [1, 1.1, 1],
            opacity: isListening ? [0.3, 0.6, 0.3] : [0.1, 0.2, 0.1]
          }}
          transition={{
            duration: 2,
            repeat: Infinity,
            ease: "easeInOut"
          }}
        />
      </div>

      <div className="relative z-10 flex flex-col items-center space-y-6">
        {/* Main voice button */}
        <motion.div
          className="relative"
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.95 }}
        >
          <motion.button
            onClick={handleToggleListening}
            className={cn(
              "relative w-24 h-24 rounded-full flex items-center justify-center transition-all duration-300",
              "bg-gradient-to-br from-accent/20 to-accent/10 border-2",
              isListening ? "border-accent shadow-lg shadow-accent/25" :
              isSpeaking ? "border-green-500 shadow-lg shadow-green-500/25" :
              "border-bg-300 hover:border-accent/50"
            )}
            animate={{
              boxShadow: isListening 
                ? ["0 0 0 0 rgba(14, 165, 233, 0.4)", "0 0 0 20px rgba(14, 165, 233, 0)"]
                : undefined
            }}
            transition={{
              duration: 1.5,
              repeat: isListening ? Infinity : 0
            }}
          >
            <AnimatePresence mode="wait">
              {isSpeaking ? (
                <motion.div
                  key="speaking"
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.8 }}
                >
                  <Volume2 className="w-10 h-10 text-green-500" />
                </motion.div>
              ) : isListening ? (
                <motion.div
                  key="listening"
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.8 }}
                >
                  <Mic className="w-10 h-10 text-accent" />
                </motion.div>
              ) : (
                <motion.div
                  key="idle"
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.8 }}
                >
                  <Mic className="w-10 h-10 text-text-400" />
                </motion.div>
              )}
            </AnimatePresence>
          </motion.button>

          {/* Pulse rings */}
          <AnimatePresence>
            {isListening && (
              <>
                <motion.div
                  className="absolute inset-0 rounded-full border-2 border-accent/30 pointer-events-none"
                  initial={{ scale: 1, opacity: 0.6 }}
                  animate={{ scale: 1.5, opacity: 0 }}
                  transition={{
                    duration: 1.5,
                    repeat: Infinity,
                    ease: "easeOut"
                  }}
                />
                <motion.div
                  className="absolute inset-0 rounded-full border-2 border-accent/20 pointer-events-none"
                  initial={{ scale: 1, opacity: 0.4 }}
                  animate={{ scale: 2, opacity: 0 }}
                  transition={{
                    duration: 1.5,
                    repeat: Infinity,
                    ease: "easeOut",
                    delay: 0.5
                  }}
                />
              </>
            )}
          </AnimatePresence>
        </motion.div>

        {/* Waveform visualizer */}
        <div className="flex items-center justify-center space-x-1 h-12">
          {waveformData.map((height, index) => (
            <motion.div
              key={index}
              className={cn(
                "w-0.5 rounded-full transition-colors duration-300",
                isListening ? "bg-accent" :
                isSpeaking ? "bg-green-500" :
                "bg-bg-300"
              )}
              animate={{
                height: `${Math.max(4, height * 0.4)}px`,
                opacity: isListening || isSpeaking ? 1 : 0.3
              }}
              transition={{
                duration: 0.1,
                ease: "easeOut"
              }}
            />
          ))}
        </div>

        {/* Status and timer */}
        <div className="text-center space-y-2">
          <motion.p
            className={cn("text-sm font-medium font-mono transition-colors", getStatusColor())}
            animate={{ opacity: [1, 0.7, 1] }}
            transition={{
              duration: 2,
              repeat: isListening || isSpeaking ? Infinity : 0
            }}
          >
            {getStatusText()}
          </motion.p>
          
          {duration > 0 && (
            <p className="text-xs text-text-500 font-mono">
              {formatTime(duration)}
            </p>
          )}

          {volume > 0 && (
            <motion.div
              className="flex items-center justify-center space-x-2"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
            >
              <VolumeX className="w-3 h-3 text-text-500" />
              <div className="w-20 h-1.5 bg-bg-300 rounded-full overflow-hidden">
                <motion.div
                  className="h-full bg-accent rounded-full"
                  animate={{ width: `${volume}%` }}
                  transition={{ duration: 0.1 }}
                />
              </div>
              <Volume2 className="w-3 h-3 text-text-500" />
            </motion.div>
          )}

          {/* Transcript preview */}
          {transcript && (
            <motion.div
              className="mt-4 max-w-xs px-4 py-2 bg-bg-200 rounded-lg border border-bg-300"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
            >
              <p className="text-xs text-text-300 line-clamp-2">
                {transcript}
              </p>
            </motion.div>
          )}
        </div>

        {/* SkyCode indicator */}
        <motion.div
          className="flex items-center space-x-2 text-xs text-text-500 font-mono"
          animate={{ opacity: [0.5, 1, 0.5] }}
          transition={{
            duration: 3,
            repeat: Infinity,
            ease: "easeInOut"
          }}
        >
          <Zap className="w-3 h-3" />
          <span>Voice Assistant</span>
        </motion.div>
      </div>
    </div>
  );
}
