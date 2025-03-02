import { motion } from 'framer-motion';
import Link from 'next/link';
import { Typewriter } from 'react-simple-typewriter';
import { Button } from "@/components/ui/button"
import ScrollIndicator from "@/components/ui/scroll-indicator";

const HeroSection = () => {
    return (
        <section className="min-h-screen flex items-center justify-center text-center">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-2 md:gap-4 xl:gap-8 items-center">
                {/* Left Column - Text Content */}
                <div className="text-center md:text-left">
                    <motion.h1
                        className="text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-7xl font-extrabold leading-tight"
                        initial={{ opacity: 0, y: -20 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ duration: 0.8 }}
                    >
                        Welcome to <br />
                        <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-purple-600">
              Decent Cloud
            </span>
                    </motion.h1>

                    <motion.p
                        className="text-base md:text-lg xl:text-xl mt-4 text-white/80"
                        animate={{ opacity: 1, y: 0 }}
                        initial={{ opacity: 0, y: 20 }}
                        transition={{ duration: 0.5, delay: 0.2 }}
                    >
                        Where the sky&apos;s not the limit, it&apos;s just the beginning!
                    </motion.p>

                    <motion.h2
                        className="text-lg md:text-2xl mt-2 text-white/80 font-bold"
                        animate={{ opacity: 1, y: 0 }}
                        initial={{ opacity: 0, y: -20 }}
                        transition={{ duration: 0.8 }}
                    >
                        <Typewriter
                            words={['Peer-to-Peer Cloud Marketplace', 'No Vendor Lock-in', 'Community-Driven', 'Liberal Open Source License']}
                            loop={true}
                            cursor
                            cursorStyle="|"
                            typeSpeed={50}
                            deleteSpeed={30}
                            delaySpeed={1000}
                        />
                    </motion.h2>

                    <motion.div
                        animate={{ opacity: 1, y: 0 }}
                        initial={{ opacity: 0, y: 20 }}
                        transition={{ duration: 0.5, delay: 0.4 }}
                        className="mt-8"
                    >
                        <Button
                            className="relative px-6 py-6 sm:px-8 md:px-10 rounded-full font-extrabold text-lg md:text-xl lg:text-2xl text-white bg-gradient-to-r from-blue-500 to-purple-600 transition-all duration-300
                          hover:brightness-110 hover:shadow-2xl hover:scale-105"
                        >
                            <Link
                                href="https://github.com/orgs/decent-stuff/discussions"
                                className="relative z-10 flex items-center gap-3"
                            >
                                <span>🚀 Join the Development</span>
                            </Link>
                            <span
                                className="absolute inset-0 bg-white/20 rounded-full scale-0 transition-transform duration-300 group-hover:scale-100"
                            ></span>
                        </Button>
                    </motion.div>
                </div>

                {/* Right Column - Image or SVG */}
                <motion.div
                    className="flex justify-center md:justify-end"
                    initial={{ opacity: 0, scale: 0.8 }}
                    animate={{ opacity: 1, scale: 1 }}
                    transition={{ duration: 0.8, delay: 0.3 }}
                >
                    <motion.img
                        src="./images/cloud-illustration.png"
                        alt=""
                        className="w-full md:min-w-[375px] max-w-md md:max-w-lg lg:max-w-xl"
                        animate={{
                            y: [0, -5, 0],
                            scale: [1, 1.05, 1],
                        }}
                        transition={{
                            duration: 5,
                            repeat: Infinity,
                            ease: 'easeInOut',
                        }}
                    />
                </motion.div>
            </div>

            <ScrollIndicator/>
        </section>
    );
};

export default HeroSection;
