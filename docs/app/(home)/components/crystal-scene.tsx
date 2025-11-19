'use client';

import { useMemo, useRef } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import { Float, Environment, ContactShadows, MeshTransmissionMaterial, PerspectiveCamera, Line } from '@react-three/drei';
import * as THREE from 'three';

type LayerProps = {
  position: [number, number, number];
  color: string;
  rotation?: [number, number, number];
  scale?: [number, number, number];
};

type WaveConfig = {
  color: string;
  speed: number;
  offsetY: number;
  amplitude: number;
  phaseOffset: number;
  zPos: number;
  width: number;
};

// --- Math Helper ---
const getWaveY = (x: number, t: number, speed: number, amplitude: number, phaseOffset: number, offsetY: number) => {
  const xEffect = x * 0.5;
  const wave1 = Math.sin(xEffect + t * speed + phaseOffset) * amplitude;
  const wave2 = Math.sin(xEffect * 2.5 + t * speed * 1.5) * (amplitude * 0.2); // Noise
  return offsetY + wave1 + wave2;
};

// --- Background Wave Components ---

const TradeMarker = ({ x, config }: { x: number; config: WaveConfig }) => {
  const meshRef = useRef<THREE.Mesh>(null);
  // Random offset for flashing so they don't all flash together
  const flashOffset = useMemo(() => Math.random() * 10, []);
  
  // Determine type randomly: Buy (Green-ish) or Sell (Red/Gold-ish)
  // We use colors that complement the scene (Cyan/Purple) but stand out.
  const type = useMemo(() => Math.random() > 0.5 ? 'buy' : 'sell', []);
  const color = type === 'buy' ? '#67e8f9' : '#fbbf24'; // Cyan-300 vs Amber-400

  useFrame((state) => {
    if (!meshRef.current) return;
    const t = state.clock.getElapsedTime();
    
    // Calculate Y position on the wave to stick to it
    const y = getWaveY(x, t, config.speed, config.amplitude, config.phaseOffset, config.offsetY);
    
    meshRef.current.position.set(x, y, config.zPos);
    
    // Flashing/Pulsing effect
    const flash = Math.sin(t * 5 + flashOffset);
    // Scale pulses between small and slightly larger
    const baseScale = 0.12;
    const scale = baseScale + (flash * 0.06); 
    
    meshRef.current.scale.setScalar(scale);
    
    // Rotate for extra dynamism (diamond spin)
    meshRef.current.rotation.y += 0.03;
    meshRef.current.rotation.z -= 0.02;
  });

  return (
    <mesh ref={meshRef}>
      <octahedronGeometry args={[1, 0]} />
      {/* Basic material for a "glowing" unlit look */}
      <meshBasicMaterial color={color} toneMapped={false} /> 
    </mesh>
  );
};

const WaveLine = ({
  color,
  speed,
  offsetY,
  amplitude,
  phaseOffset,
  zPos,
  width,
}: WaveConfig) => {
  const lineRef = useRef<any>(null);
  
  const count = 60;
  const xRange = 40;
  const startX = -20;

  const initialPoints = useMemo(() => {
    return new Array(count).fill(0).map((_, i) => {
      const x = startX + (i / (count - 1)) * xRange;
      return new THREE.Vector3(x, offsetY, zPos);
    });
  }, [offsetY, zPos]);

  useFrame((state) => {
    if (!lineRef.current || !lineRef.current.geometry) return;
    const t = state.clock.getElapsedTime();
    
    const positions: number[] = [];
    
    for (let i = 0; i < count; i++) {
      const x = startX + (i / (count - 1)) * xRange;
      const y = getWaveY(x, t, speed, amplitude, phaseOffset, offsetY);
      positions.push(x, y, zPos);
    }
    
    lineRef.current.geometry.setPositions(positions);
  });

  return (
    <Line
      ref={lineRef}
      points={initialPoints}
      color={color}
      lineWidth={width}
      transparent
      opacity={0.6}
    />
  );
};

const WaveSystem = (props: WaveConfig) => {
  // Generate pseudo-random markers for this wave
  const markers = useMemo(() => {
    const count = Math.floor(Math.random() * 3) + 2; // 2 to 4 markers per wave
    const positions = [];
    for(let i=0; i<count; i++) {
        // Spread them randomly across the visible width (-12 to 12)
        positions.push((Math.random() - 0.5) * 24); 
    }
    return positions;
  }, []);

  return (
    <>
      <WaveLine {...props} />
      {markers.map((x, i) => (
        <TradeMarker key={i} x={x} config={props} />
      ))}
    </>
  );
};

const BackgroundWaves = () => {
  return (
    <group position={[0, 0, -8]}>
      {/* Cyan / Blue Trend */}
      <WaveSystem color="#22d3ee" speed={0.5} offsetY={3} amplitude={1.5} phaseOffset={0} zPos={0} width={2} />
      <WaveSystem color="#0ea5e9" speed={0.6} offsetY={1} amplitude={1.2} phaseOffset={1} zPos={-2} width={1.5} />
      
      {/* Purple / Pink Trend */}
      <WaveSystem color="#d946ef" speed={0.4} offsetY={-1} amplitude={1.8} phaseOffset={2} zPos={-1} width={2.5} />
      <WaveSystem color="#a855f7" speed={0.7} offsetY={-3} amplitude={1.0} phaseOffset={3} zPos={1} width={1.5} />
      
      {/* Subtle background filler */}
      <WaveSystem color="#4f46e5" speed={0.3} offsetY={0} amplitude={2.5} phaseOffset={4} zPos={-4} width={1} />
    </group>
  );
};

// --- Crystal Logic ---

const CrystalLayer = ({ position, color, rotation = [0, 0, Math.PI / 4], scale = [1.8, 1.8, 0.3] }: LayerProps) => {
  const meshRef = useRef<THREE.Mesh>(null);

  useFrame((state) => {
    if (!meshRef.current) return;
    const t = state.clock.getElapsedTime();
    // Subtle floating relative to its container
    meshRef.current.position.y = position[1] + Math.sin(t * 0.5 + position[2]) * 0.05;
  });

  const config = useMemo(() => ({
    meshPhysicalMaterial: false,
    transmissionSampler: false,
    backside: false,
    samples: 8,
    resolution: 1024,
    transmission: 1,
    roughness: 0.0, 
    thickness: 1.2,
    ior: 1.6,
    chromaticAberration: 0.08,
    anisotropy: 0.2,
    distortion: 0.1,
    distortionScale: 0.3,
    temporalDistortion: 0.2,
    clearcoat: 1,
    attenuationDistance: 4,
    attenuationColor: color,
    color: color,
    bg: '#020617'
  }), [color]);

  return (
    <mesh ref={meshRef} position={position} rotation={new THREE.Euler(...rotation)} castShadow receiveShadow>
      <boxGeometry args={[scale[0], scale[1], scale[2]]} />
      {/* @ts-ignore */}
      <MeshTransmissionMaterial {...config} background={new THREE.Color(config.bg)} />
    </mesh>
  );
};

const LogoGroup = () => {
  const groupRef = useRef<THREE.Group>(null);
  const { viewport } = useThree();
  
  // Simple responsive check: if viewport width > 12 units (roughly desktop), shift right
  const isDesktop = viewport.width > 12;
  const targetX = isDesktop ? 3.5 : 0;
  const targetY = isDesktop ? 0 : 1.5; // Lift up slightly on mobile

  useFrame((state) => {
    if (!groupRef.current) return;
    const t = state.clock.getElapsedTime();
    
    // Gentle sway
    groupRef.current.rotation.y = Math.sin(t * 0.2) * 0.15;
    groupRef.current.rotation.x = Math.sin(t * 0.15) * 0.08;
    
    // Smooth lerp to target position for responsive transitions
    groupRef.current.position.x = THREE.MathUtils.lerp(groupRef.current.position.x, targetX, 0.05);
    groupRef.current.position.y = THREE.MathUtils.lerp(groupRef.current.position.y, targetY, 0.05);
  });

  return (
    <group ref={groupRef}>
      {/* 
         Z-AXIS STACKING CONFIGURATION
         Layers are positioned close in Y (virtually overlapping from front)
         but separated significantly in Z to create depth.
      */}

      {/* Front Layer - Pink - Closest to camera */}
      <CrystalLayer 
        position={[0, 0.2, 1.5]} 
        rotation={[0, 0, Math.PI / 4]}
        scale={[1.6, 1.6, 0.25]} 
        color="#f0abfc" 
      />
      
      {/* Middle Layer - Deep Violet - Center */}
      <CrystalLayer 
        position={[0, 0, 0]} 
        rotation={[0, 0, Math.PI / 4]}
        scale={[1.8, 1.8, 0.25]}
        color="#8b5cf6" 
      />
      
      {/* Back Layer - Blue - Furthest away */}
      <CrystalLayer 
        position={[0, -0.2, -1.5]} 
        rotation={[0, 0, Math.PI / 4]}
        scale={[2.0, 2.0, 0.25]} 
        color="#60a5fa" 
      />
    </group>
  );
};

export const CrystalScene = () => {
  return (
    <>
      {/* Camera moved back to accommodate Z-depth */}
      <PerspectiveCamera makeDefault position={[0, 0, 14]} fov={40} />
      
      {/* Background Elements */}
      <BackgroundWaves />
      
      {/* Lighting */}
      <ambientLight intensity={1.5} />
      <spotLight position={[10, 10, 10]} angle={0.3} penumbra={1} intensity={8} castShadow />
      <pointLight position={[-10, -5, -10]} intensity={5} color="#a855f7" />
      <pointLight position={[5, 0, 5]} intensity={4} color="#ffffff" />
      <spotLight position={[-5, 8, -5]} intensity={10} angle={0.5} penumbra={0.5} color="#e879f9" />

      <Environment preset="city" environmentIntensity={1.5} />

      <Float
        speed={2} 
        rotationIntensity={0.2} 
        floatIntensity={0.4} 
        floatingRange={[-0.1, 0.1]}
      >
        <LogoGroup />
      </Float>

      {/* Shadow adjusted for new position */}
      <ContactShadows 
        position={[0, -3.5, 0]} 
        opacity={0.5} 
        scale={30} 
        blur={2.5} 
        far={4.5} 
        color="#4c1d95"
      />
    </>
  );
};
