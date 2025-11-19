'use client';

import Link from 'next/link';
import { type ReactNode, useEffect, useState } from 'react';
import { 
  ArrowRight, Zap, Cpu, Globe, Terminal, 
  Activity, Lock, Github, LayoutGrid, RefreshCw, 
  Server, ShieldAlert, Boxes, BrainCircuit, TrendingUp,
  Code2
} from 'lucide-react';
import { motion } from 'framer-motion';
import { cn } from '@/lib/utils';

// --- Components ---

const Badge = ({ children, color = "blue" }: { children: ReactNode, color?: "blue" | "purple" | "green" | "amber" }) => {
  const colors = {
    blue: "bg-blue-500/10 text-blue-400 border-blue-500/20",
    purple: "bg-purple-500/10 text-purple-400 border-purple-500/20",
    green: "bg-green-500/10 text-green-400 border-green-500/20",
    amber: "bg-amber-500/10 text-amber-400 border-amber-500/20",
  };
  
  return (
    <span className={cn("px-3 py-1 rounded-full text-xs font-medium border uppercase tracking-wider", colors[color])}>
      {children}
    </span>
  );
};

const SectionHeading = ({ title, subtitle, align = "center" }: { title: ReactNode, subtitle?: string, align?: "left" | "center" }) => (
  <div className={cn("mb-16", align === "center" ? "text-center" : "text-left")}>
    <motion.h2 
      initial={{ opacity: 0, y: 20 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true }}
      className="text-3xl md:text-5xl font-bold text-white mb-4 tracking-tight"
    >
      {title}
    </motion.h2>
    {subtitle && (
      <motion.p 
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ delay: 0.1 }}
        className="text-zinc-400 text-lg md:text-xl max-w-2xl mx-auto leading-relaxed"
      >
        {subtitle}
      </motion.p>
    )}
  </div>
);

// --- Sections ---

function HeroSection() {
  return (
    <section className="relative h-screen flex items-center overflow-hidden pointer-events-none">
      <div className="container mx-auto px-6 md:px-12 relative z-10 pointer-events-auto">
        <div className="flex flex-col items-start text-left max-w-4xl">
          
          <motion.div 
            initial={{ opacity: 0, x: -20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.5 }}
            className="flex items-center gap-3 mb-6"
          >
            <div className="flex items-center gap-2 px-3 py-1 rounded-full bg-zinc-900/80 border border-zinc-800 backdrop-blur-md">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
              </span>
              <span className="text-xs font-mono text-zinc-400">SYSTEM OPERATIONAL</span>
            </div>
            <span className="text-xs font-mono text-zinc-600">V0.2.3-RC1</span>
          </motion.div>

          <motion.h1 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className="text-5xl md:text-7xl lg:text-8xl font-bold tracking-tighter text-white mb-6 drop-shadow-2xl leading-[0.95]"
          >
            The Operating System for <br/>
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 via-purple-400 to-cyan-400">
              Quantitative Finance
            </span>
          </motion.h1>
          
          <motion.p 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.2 }}
            className="text-xl md:text-2xl text-zinc-400 max-w-2xl mb-10 leading-relaxed font-light"
          >
            Eliminate the gap between <span className="text-white font-medium">research</span> and <span className="text-white font-medium">execution</span>. 
            <br className="hidden md:block"/> The institutional-grade infrastructure standard for the next generation of funds.
          </motion.p>
          
          <motion.div 
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.3 }}
            className="flex flex-wrap items-center gap-4"
          >
            <Link
              href="/docs"
              className="group relative px-8 py-4 bg-white text-black font-bold rounded-full overflow-hidden transition-all hover:scale-105"
            >
              <div className="absolute inset-0 bg-gradient-to-r from-blue-400 to-cyan-300 opacity-0 group-hover:opacity-20 transition-opacity" />
              <span className="flex items-center gap-2">
                Deploy Core <ArrowRight className="size-4" />
              </span>
            </Link>
            
            <a
              href="https://github.com/tesserspace/tesser"
              target="_blank"
              rel="noreferrer"
              className="flex items-center gap-2 px-8 py-4 rounded-full bg-black/40 border border-white/10 text-white font-medium backdrop-blur-md hover:bg-white/5 transition-all"
            >
              <Github className="size-4" /> Star on GitHub
            </a>
          </motion.div>
        </div>
      </div>

      {/* Tech Details Overlay */}
      <div className="absolute bottom-10 left-6 md:left-12 flex gap-8 text-xs font-mono text-zinc-500 z-10">
        <div className="flex items-center gap-2">
          <Cpu className="size-3" />
          <span>RUST CORE ENGINE</span>
        </div>
        <div className="flex items-center gap-2">
          <Zap className="size-3" />
          <span>&lt; 50μs LATENCY</span>
        </div>
        <div className="flex items-center gap-2">
          <Activity className="size-3" />
          <span>100% MEMORY SAFE</span>
        </div>
      </div>
    </section>
  );
}

function ProblemSection() {
  return (
    <section className="py-32 bg-zinc-950 relative z-20 overflow-hidden">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-purple-900/20 via-black to-black opacity-50" />
      
      <div className="container mx-auto px-6 md:px-12 relative">
        <SectionHeading 
          title={<span>The <span className="text-red-500">Impossible Triangle</span> of Quant</span>}
          subtitle="Institutions today are forced to compromise. We believe you shouldn't have to."
        />

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          {[
            {
              icon: <LayoutGrid className="size-6 text-red-400" />,
              title: "Speed vs. Agility",
              desc: "C++ is fast but slow to write. Python is agile but too slow for execution. You're either losing alpha to latency or losing opportunity to dev time."
            },
            {
              icon: <RefreshCw className="size-6 text-amber-400" />,
              title: "The Research Gap",
              desc: "Backtests are written in Python, execution in Java/C++. Translating strategies creates logic drift, leading to 'profitable backtest, bankrupt reality'."
            },
            {
              icon: <Server className="size-6 text-blue-400" />,
              title: "Infrastructure Tax",
              desc: "Maintaining exchange connectors, websocket management, and hardware colocation drains 30% of a fund's resources away from actual alpha generation."
            }
          ].map((item, i) => (
            <motion.div 
              key={i}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ delay: i * 0.1 }}
              className="p-8 rounded-3xl bg-zinc-900/50 border border-zinc-800 backdrop-blur-sm hover:bg-zinc-900/80 hover:border-zinc-700 transition-all group"
            >
              <div className="w-12 h-12 rounded-2xl bg-zinc-800/50 flex items-center justify-center mb-6 border border-zinc-700 group-hover:border-zinc-600 transition-colors">
                {item.icon}
              </div>
              <h3 className="text-xl font-bold text-white mb-3">{item.title}</h3>
              <p className="text-zinc-400 leading-relaxed text-sm">{item.desc}</p>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}

function SolutionArchitecture() {
  const [activeStep, setActiveStep] = useState(0);
  
  useEffect(() => {
    const interval = setInterval(() => {
      setActiveStep(prev => (prev + 1) % 3);
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  return (
    <section className="py-32 bg-black border-y border-zinc-900 relative z-20">
      <div className="absolute inset-0 bg-grid-pattern bg-[size:40px_40px] opacity-[0.05]" />
      
      <div className="container mx-auto px-6 md:px-12">
        <div className="flex flex-col lg:flex-row gap-16 items-center">
          
          {/* Left Text */}
          <div className="flex-1 space-y-8">
            <Badge color="blue">Architecture</Badge>
            <h2 className="text-4xl md:text-5xl font-bold text-white leading-tight">
              Iron Core, <br />
              <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-500 to-cyan-400">
                Fluid Exterior.
              </span>
            </h2>
            <p className="text-zinc-400 text-lg leading-relaxed">
              Tesser implements a revolutionary <strong>Hybrid Architecture</strong>. The execution engine is pure Rust for safety and speed, while the user interface is exposed via gRPC to Python/Go/JS.
            </p>
            
            <div className="space-y-6 pt-4">
              {[
                { title: "Rust Core", desc: "Order routing, risk checks, and event loop run on bare metal.", active: activeStep === 0 },
                { title: "Polyglot SDK", desc: "Write strategies in Python, JavaScript or Go. Zero learning curve.", active: activeStep === 1 },
                { title: "Unified Event Bus", desc: "Deterministic replay of market events ensures 100% backtest accuracy.", active: activeStep === 2 },
              ].map((item, i) => (
                <div 
                  key={i} 
                  className={cn(
                    "pl-6 border-l-2 transition-all duration-500 cursor-pointer",
                    item.active ? "border-blue-500" : "border-zinc-800 opacity-40 hover:opacity-70"
                  )}
                  onClick={() => setActiveStep(i)}
                >
                  <h4 className={cn("text-lg font-bold mb-1", item.active ? "text-white" : "text-zinc-300")}>{item.title}</h4>
                  <p className="text-sm text-zinc-400">{item.desc}</p>
                </div>
              ))}
            </div>
          </div>

          {/* Right Graphic */}
          <div className="flex-1 w-full">
            <div className="relative rounded-2xl bg-[#0c0c0e] border border-zinc-800 shadow-2xl overflow-hidden p-1">
              {/* Window Header */}
              <div className="flex items-center gap-2 px-4 py-3 bg-zinc-900/50 border-b border-zinc-800">
                <div className="flex gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-red-500/20 border border-red-500/50" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500/20 border border-yellow-500/50" />
                  <div className="w-3 h-3 rounded-full bg-green-500/20 border border-green-500/50" />
                </div>
                <div className="ml-auto text-xs font-mono text-zinc-500">tesser-engine — rust — 80x24</div>
              </div>
              
              {/* Code Content */}
              <div className="p-6 font-mono text-xs md:text-sm relative h-[320px]">
                {activeStep === 0 && (
                  <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-blue-300 space-y-2">
                    <div><span className="text-purple-400">fn</span> <span className="text-yellow-200">handle_market_event</span>(ev: &Event) -&gt; Result {`{`}</div>
                    <div className="pl-4 text-zinc-500">// Zero-copy deserialization</div>
                    <div className="pl-4">let quote = ev.as_quote().unsafe_cast();</div>
                    <div className="pl-4 mb-2"></div>
                    <div className="pl-4"><span className="text-purple-400">match</span> self.risk_manager.check(quote) {`{`}</div>
                    <div className="pl-8">Ok(_) =&gt; self.order_book.update(quote),</div>
                    <div className="pl-8">
                      Err(e) =&gt; <span className="text-red-400">log::error!</span>{'("Risk Reject: {:?}", e),'}
                    </div>
                    <div className="pl-4">{`}`}</div>
                    <div>{`}`}</div>
                    <div className="mt-4 text-green-400/80">
                      &gt; [CORE] System Latency: 12μs <br/>
                      &gt; [CORE] GC Overhead: 0ms (Rust)
                    </div>
                  </motion.div>
                )}
                
                {activeStep === 1 && (
                  <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-yellow-100 space-y-2">
                    <div className="text-zinc-500"># strategy.py - High level abstraction</div>
                    <div><span className="text-purple-400">class</span> <span className="text-green-300">MeanReversion</span>(Strategy):</div>
                    <div className="pl-4"><span className="text-purple-400">async def</span> <span className="text-blue-300">on_tick</span>(self, tick):</div>
                    <div className="pl-8">sma = self.indicators.sma(period=20)</div>
                    <div className="pl-8"><span className="text-purple-400">if</span> tick.price &lt; sma * 0.98:</div>
                    <div className="pl-12 text-zinc-500"># Sends gRPC command to Rust Core</div>
                    <div className="pl-12"><span className="text-purple-400">await</span> self.buy(limit=tick.price)</div>
                    <div className="mt-4 text-blue-400">
                      &gt; [SDK] Connected to local Tesser Core<br/>
                      &gt; [SDK] Subscribed to 'binance:btcusdt'
                    </div>
                  </motion.div>
                )}

                {activeStep === 2 && (
                  <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-zinc-300 space-y-2">
                    <div className="flex items-center gap-2 text-green-400 mb-4">
                      <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"/>
                      REPLAY MODE ACTIVE
                    </div>
                    <div className="grid grid-cols-3 gap-4 text-center text-xs mb-6">
                      <div className="p-2 bg-zinc-800/50 rounded border border-zinc-700">
                        <div className="text-zinc-500">Events</div>
                        <div className="font-bold text-white">14,203,921</div>
                      </div>
                      <div className="p-2 bg-zinc-800/50 rounded border border-zinc-700">
                        <div className="text-zinc-500">Speed</div>
                        <div className="font-bold text-blue-400">12,000x</div>
                      </div>
                      <div className="p-2 bg-zinc-800/50 rounded border border-zinc-700">
                        <div className="text-zinc-500">Drift</div>
                        <div className="font-bold text-green-400">0.00%</div>
                      </div>
                    </div>
                    <div className="h-1 bg-zinc-800 rounded-full overflow-hidden">
                      <motion.div 
                        className="h-full bg-blue-500"
                        initial={{ width: "0%" }}
                        animate={{ width: "75%" }}
                        transition={{ duration: 2 }}
                      />
                    </div>
                    <div className="mt-2 text-zinc-500 font-mono text-[10px]">
                      Processing 2023-11-04T12:00:00Z chunk...
                    </div>
                  </motion.div>
                )}

                {/* Glowing Effect */}
                <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-64 bg-blue-500/20 blur-[80px] rounded-full pointer-events-none mix-blend-screen" />
              </div>
            </div>
          </div>

        </div>
      </div>
    </section>
  );
}

function FeatureGrid() {
  return (
    <section className="py-32 bg-zinc-950 relative z-20">
      <div className="container mx-auto px-6 md:px-12">
        <SectionHeading 
          title="Strategic Advantages"
          subtitle="Built for funds that manage AUM, not just play money."
        />

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 auto-rows-[minmax(250px,auto)]">
          
          {/* Large Card - Isomorphism */}
          <div className="md:col-span-2 row-span-1 relative group overflow-hidden rounded-3xl bg-gradient-to-b from-zinc-900 to-zinc-950 border border-zinc-800 p-8">
            <div className="absolute right-0 top-0 p-32 bg-blue-500/10 blur-[60px] rounded-full pointer-events-none transition-opacity group-hover:opacity-70" />
            <div className="relative z-10">
              <div className="w-10 h-10 bg-blue-500/20 rounded-lg flex items-center justify-center mb-4 border border-blue-500/30">
                <Boxes className="size-5 text-blue-400" />
              </div>
              <h3 className="text-2xl font-bold text-white mb-2">True Isomorphism</h3>
              <p className="text-zinc-400">
                What you see in backtest is <i>exactly</i> what you get in production. Tesser replays historical raw websocket frames through the exact same binary that executes live trades. No simulation logic drift.
              </p>
            </div>
          </div>

          {/* Chaos Engineering */}
          <div className="md:col-span-2 row-span-1 relative group overflow-hidden rounded-3xl bg-zinc-900 border border-zinc-800 p-8 hover:border-zinc-700 transition-colors">
             <div className="absolute right-0 bottom-0 p-24 bg-red-500/5 blur-[50px] rounded-full pointer-events-none" />
             <div className="relative z-10">
              <div className="w-10 h-10 bg-red-500/20 rounded-lg flex items-center justify-center mb-4 border border-red-500/30">
                <ShieldAlert className="size-5 text-red-400" />
              </div>
              <h3 className="text-2xl font-bold text-white mb-2">Chaos Engineering</h3>
              <p className="text-zinc-400">
                Born in fire. Tesser is continuously tested against simulated exchange outages, API rate limits, and network partitions. We crash it so the market can't.
              </p>
             </div>
          </div>

          {/* Cloud Native */}
          <div className="md:col-span-1 relative group rounded-3xl bg-zinc-900 border border-zinc-800 p-6">
             <Globe className="size-8 text-purple-500 mb-4" />
             <h3 className="text-lg font-bold text-white mb-2">Cloud Native</h3>
             <p className="text-sm text-zinc-400">Distributed architecture allows strategy nodes to run on AWS while execution nodes sit in Tokyo colocation.</p>
          </div>

          {/* Risk Engine */}
          <div className="md:col-span-1 relative group rounded-3xl bg-zinc-900 border border-zinc-800 p-6">
             <Lock className="size-8 text-green-500 mb-4" />
             <h3 className="text-lg font-bold text-white mb-2">Pre-Trade Risk</h3>
             <p className="text-sm text-zinc-400">Atomic risk checks prevent "fat finger" errors and enforce portfolio-wide drawdown limits in microseconds.</p>
          </div>
          
          {/* Extensible */}
          <div className="md:col-span-2 relative group rounded-3xl bg-zinc-900 border border-zinc-800 p-6 flex items-center gap-6">
             <div className="flex-1">
                <h3 className="text-lg font-bold text-white mb-2">Open Protocol</h3>
                <p className="text-sm text-zinc-400">Don't like our Python SDK? Write your own in C#, Java or Haskell using our documented gRPC protos.</p>
             </div>
             <Code2 className="size-12 text-zinc-700" />
          </div>

        </div>
      </div>
    </section>
  );
}

function RoadmapSection() {
  return (
    <section className="py-32 bg-black border-t border-zinc-900 relative z-20">
      <div className="container mx-auto px-6 md:px-12">
        <div className="flex flex-col md:flex-row justify-between items-end mb-20 gap-6">
           <div className="max-w-2xl">
            <h2 className="text-4xl font-bold text-white mb-4">The Future of Algo-Trading</h2>
            <p className="text-zinc-400 text-lg">We are not just building a library; we are defining the standard protocol for algorithmic liquidity.</p>
           </div>
           <a 
             href="https://github.com/tesserspace/tesser" 
             target="_blank" 
             rel="noreferrer" 
             className="text-blue-400 hover:text-blue-300 font-mono text-sm flex items-center gap-2"
           >
             VIEW FULL ROADMAP <ArrowRight className="size-3"/>
           </a>
        </div>

        <div className="relative">
          {/* Connecting Line */}
          <div className="absolute top-[2rem] left-0 w-full h-px bg-gradient-to-r from-blue-500/50 via-zinc-800 to-zinc-900 hidden md:block" />

          <div className="grid grid-cols-1 md:grid-cols-3 gap-12">
            {[
              {
                phase: "Phase 1: Standardization",
                status: "current",
                icon: <Terminal className="size-5 text-white"/>,
                items: ["Core Engine Stability", "Python/Rust Bridge", "Bybit & Binance Linear"]
              },
              {
                phase: "Phase 2: Intelligence",
                status: "upcoming",
                icon: <BrainCircuit className="size-5 text-zinc-400"/>,
                items: ["Native PyTorch/JAX Streams", "AI Model Inference Node", "Sentiment Analysis Oracle"]
              },
              {
                phase: "Phase 3: Marketplace",
                status: "future",
                icon: <TrendingUp className="size-5 text-zinc-400"/>,
                items: ["DeFi/CeFi Aggregation", "Strategy Marketplace", "Decentralized Alpha Protocol"]
              }
            ].map((item, i) => (
               <div key={i} className="relative pt-8 md:pt-12 group">
                  {/* Dot */}
                  <div className={cn(
                    "absolute top-0 left-0 md:left-0 w-4 h-4 rounded-full border-2 z-10 bg-black transition-all duration-500",
                    item.status === 'current' ? "border-blue-500 shadow-[0_0_20px_rgba(59,130,246,0.5)] scale-125" : "border-zinc-700 group-hover:border-zinc-500"
                  )} />
                  
                  <h3 className={cn("text-xl font-bold mb-4", item.status === 'current' ? "text-white" : "text-zinc-500")}>
                    {item.phase}
                  </h3>
                  
                  <ul className="space-y-3">
                    {item.items.map((feat, j) => (
                      <li key={j} className="flex items-center gap-3 text-sm text-zinc-400">
                        <div className="w-1 h-1 bg-zinc-600 rounded-full" />
                        {feat}
                      </li>
                    ))}
                  </ul>
               </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

function CTASection() {
  return (
    <section className="py-32 relative overflow-hidden z-20">
      {/* Background Atmosphere */}
      <div className="absolute inset-0 bg-gradient-to-b from-black to-indigo-950/30" />
      <div className="absolute top-0 left-1/2 -translate-x-1/2 w-full max-w-[800px] h-[400px] bg-blue-600/10 blur-[120px] rounded-full pointer-events-none" />
      
      <div className="container mx-auto px-6 md:px-12 relative z-10 text-center">
        <h2 className="text-5xl md:text-7xl font-bold text-white mb-8 tracking-tighter">
          Ready to <br/>
          <span className="text-transparent bg-clip-text bg-gradient-to-b from-white to-white/40">
            Professionalize?
          </span>
        </h2>
        <p className="text-zinc-400 text-lg max-w-xl mx-auto mb-12">
          Join the community of quants building the next generation of trading infrastructure. Open source and free for individuals.
        </p>
        
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
          <Link 
            href="/docs"
            className="px-8 py-4 rounded-full bg-white text-black font-bold text-lg hover:bg-zinc-200 transition-all hover:scale-105 shadow-[0_0_40px_-10px_rgba(255,255,255,0.3)]"
          >
            Get Started Now
          </Link>
          <Link 
            href="/docs/getting-started"
            className="px-8 py-4 rounded-full bg-black border border-zinc-800 text-white font-medium text-lg hover:bg-zinc-900 transition-all flex items-center gap-2"
          >
            <Terminal className="size-5" /> Read the Docs
          </Link>
        </div>
        
        <div className="mt-16 pt-8 border-t border-white/5 flex flex-wrap justify-center gap-8 opacity-50 grayscale hover:grayscale-0 transition-all duration-500">
           {/* Fake logos for social proof vibe */}
           <span className="text-lg font-bold font-mono text-zinc-500">BYBIT</span>
           <span className="text-lg font-bold font-mono text-zinc-500">BINANCE</span>
           <span className="text-lg font-bold font-mono text-zinc-500">DYDX</span>
           <span className="text-lg font-bold font-mono text-zinc-500">AWS</span>
        </div>
      </div>
    </section>
  );
}

export function UIOverlay() {
  return (
    <main className="flex flex-col w-full min-h-screen font-sans selection:bg-blue-500/30 selection:text-white">
      <nav className="fixed top-0 left-0 w-full z-50 flex justify-between items-center px-6 py-6 md:px-12 pointer-events-none mix-blend-difference text-white">
        <div className="pointer-events-auto flex items-center gap-2">
           <div className="w-8 h-8 bg-white rounded-lg flex items-center justify-center">
              <div className="w-4 h-4 bg-black rounded-sm rotate-45" />
           </div>
           <span className="font-bold text-xl tracking-tight">tesser</span>
        </div>
        <div className="pointer-events-auto hidden md:flex gap-8 text-sm font-medium">
          <Link href="/docs" className="hover:opacity-70 transition-opacity">Documentation</Link>
          <a 
            href="https://github.com/tesserspace/tesser" 
            target="_blank" 
            rel="noreferrer" 
            className="hover:opacity-70 transition-opacity"
          >
            GitHub
          </a>
          <Link href="/blog" className="hover:opacity-70 transition-opacity">Blog</Link>
        </div>
        <div className="pointer-events-auto">
          <Link 
            href="/docs" 
            className="hidden md:block px-5 py-2 text-sm font-bold border border-white/20 rounded-full hover:bg-white hover:text-black transition-all"
          >
            Launch App
          </Link>
        </div>
      </nav>

      <HeroSection />
      <ProblemSection />
      <SolutionArchitecture />
      <FeatureGrid />
      <RoadmapSection />
      <CTASection />
      
      <footer className="py-8 bg-black border-t border-zinc-900 text-center text-zinc-600 text-sm relative z-20">
        <p>© 2024 Tesser Inc. Open Source Software. MIT License.</p>
      </footer>
    </main>
  );
}
