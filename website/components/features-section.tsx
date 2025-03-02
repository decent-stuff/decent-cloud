import React, {useEffect, useState, useRef} from "react";
import HeaderSection from "@/components/ui/header";

const features = [
    {
        icon: "🌐",
        title: "Decentralized Physical Infrastructure (DePIN)",
        description: "Access tailored virtual or physical servers from reputable node providers. It's not just a cloud, it's a whole sky full of possibilities!"
    },
    {
        icon: "⭐",
        title: "Reputation-Based System",
        description: "Make informed decisions with our transparent provider reputation system. We put the 'trust' in trustless technology!"
    },
    {
        icon: "🔒",
        title: "Confidential Computing",
        description: "Process sensitive data securely in Confidential Computing VMs. Your secrets are safe with us (even we don't know them)!"
    },
    {
        icon: "🤝",
        title: "No Vendor Lock-in",
        description: "Easy multi-cloud deployments with consistent APIs and liberal Open Source license. Decent Cloud is going nowhere, you're safe with us. You're not just a customer, you're a free spirit!"
    }
];

const FeaturesSection = () => {
    const scrollContainerRef = useRef<HTMLDivElement>(null);
    const [isPaused, setIsPaused] = useState(false);
    const [scrollPosition, setScrollPosition] = useState(0);

    // Track touch movement for manual scrolling
    const [touchStartX, setTouchStartX] = useState<number | null>(null);

    useEffect(() => {
        if (!scrollContainerRef.current) return;

        const container = scrollContainerRef.current;

        const handleTouchStart = (e: TouchEvent) => {
            setIsPaused(true);
            setTouchStartX(e.touches[0].clientX);
        };

        const handleTouchMove = (e: TouchEvent) => {
            if (isPaused && touchStartX !== null) {
                const touchDelta = e.touches[0].clientX - touchStartX;
                setScrollPosition(prev => prev + touchDelta);
                setTouchStartX(e.touches[0].clientX);
            }
        };

        const handleTouchEnd = () => {
            setIsPaused(false);
            setTouchStartX(null);
            setScrollPosition(0);
        };

        // Add event listeners for mobile touch interactions
        container.addEventListener('touchstart', handleTouchStart);
        container.addEventListener('touchmove', handleTouchMove);
        container.addEventListener('touchend', handleTouchEnd);

        return () => {
            // Cleanup event listeners
            container.removeEventListener('touchstart', handleTouchStart);
            container.removeEventListener('touchmove', handleTouchMove);
            container.removeEventListener('touchend', handleTouchEnd);
        };
    }, [isPaused, touchStartX]);

    return (
        <section id="features" className="pt-20 text-white">
            <div className="container mx-auto px-6 text-center">
                {/* Section Header */}
                <HeaderSection
                    title="Key Features"
                    subtitle="Explore the unique features that make Decent Cloud your top choice for decentralized solutions."
                />

                {/* Infinite Scrolling Slider */}
                <div
                    className="relative overflow-hidden [mask-image:linear-gradient(to_right,transparent,black_10%,black_90%,transparent)] py-12 -my-4"
                >
                    <div
                        ref={scrollContainerRef}
                        className={`flex gap-8 flex-nowrap w-max ${isPaused ? 'overflow-x-auto no-animation' : 'overflow-hidden'}`}
                        style={{
                            transform: isPaused ? `translateX(${scrollPosition}px)` : undefined,
                            transition: isPaused ? 'none' : 'transform 0.3s ease'
                        }}
                        onMouseEnter={() => setIsPaused(true)}
                        onMouseLeave={() => {
                            setIsPaused(false);
                            setScrollPosition(0);
                        }}
                        onWheel={(e) => {
                            if (isPaused && scrollContainerRef.current) {
                                e.preventDefault();
                                const newPosition = scrollPosition - e.deltaY;
                                setScrollPosition(newPosition);
                            }
                        }}
                    >
                        {[...Array(3)].map((_, idx) => (
                            <React.Fragment key={idx}>
                                {features.map((feature, index) => (
                                    <div
                                        key={index}
                                        className="feature-card w-80 border border-white/10 relative flex flex-col  bg-gradient-to-r from-gray-900/50 to-gray-700/50 rounded-xl p-6 shadow-lg  transition duration-300 ease-in-out cursor-help hover:bg-opacity-20 hover:shadow-xl hover:scale-105"
                                    >
                                        <div className="text-5xl mb-4 text-blue-400">{feature.icon}</div>
                                        <h4 className="text-2xl font-bold mb-3">{feature.title}</h4>
                                        <p className="text-gray-300">{feature.description}</p>
                                    </div>
                                ))}
                            </React.Fragment>
                        ))}
                    </div>
                </div>
            </div>

            {/* Tailwind animation styles */}
            <style jsx>{`
                @keyframes move-left {
                    0% {
                        transform: translateX(0%);
                    }
                    100% {
                        transform: translateX(-33.33%);
                    }
                }

                .flex:not(.no-animation) {
                    animation: move-left 30s linear infinite;
                }

                .flex {
                    scrollbar-width: thin;
                    scrollbar-color: rgba(255, 255, 255, 0.3) transparent;
                }

                .flex::-webkit-scrollbar {
                    height: 6px;
                }

                .flex::-webkit-scrollbar-track {
                    background: transparent;
                }

                .flex::-webkit-scrollbar-thumb {
                    background-color: rgba(255, 255, 255, 0.3);
                    border-radius: 10px;
                }
            `}</style>
        </section>
    );
};

export default FeaturesSection;
