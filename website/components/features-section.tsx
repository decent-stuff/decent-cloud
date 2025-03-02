import React, {useEffect, useState} from "react";
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

    const [isPaused, setIsPaused] = useState(false);

    useEffect(() => {
        const handleTouchStart = () => setIsPaused(true);
        const handleTouchEnd = () => setIsPaused(false);

        // Add event listeners for mobile touch interactions
        const cards = document.querySelectorAll('.feature-card');
        cards.forEach((card) => {
            card.addEventListener('touchstart', handleTouchStart);
            card.addEventListener('touchend', handleTouchEnd);
        });

        return () => {
            // Cleanup event listeners
            cards.forEach((card) => {
                card.removeEventListener('touchstart', handleTouchStart);
                card.removeEventListener('touchend', handleTouchEnd);
            });
        };
    }, []);

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
                        className="flex gap-8 flex-nowrap w-max"
                        style={{
                            animationPlayState: isPaused ? 'paused' : 'running'
                        }}
                        onMouseEnter={() => setIsPaused(true)}
                        onMouseLeave={() => setIsPaused(false)}
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

                .flex {
                    animation: move-left 30s linear infinite;
                }
            `}</style>
        </section>
    );
};

export default FeaturesSection;
