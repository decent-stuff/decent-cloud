"use client";

import { motion } from "framer-motion";
import HeaderSection from "@/components/ui/header";
import {Button} from "@headlessui/react";
import Link from "next/link";

const BenefitsSection = () => {
    return (
        <section id="benefits" className="py-20">
            <div className="container mx-auto px-6 text-center">
                <HeaderSection
                    title={"Benefits"}
                    subtitle={"Unlock the advantages for developers and providers in the decentralized cloud"}
                />
                <motion.div
                    className="max-w-4xl mx-auto grid grid-cols-1 gap-8"
                    initial={{opacity: 0, y: 20}}
                    whileInView={{opacity: 1, y: 0}}
                    viewport={{once: true, amount: 0.3}}
                    transition={{duration: 0.6}}
                >
                    <motion.div
                        className="bg-gradient-to-r from-gray-800/30 to-gray-700/30 border border-white/10
                           p-6 rounded-xl backdrop-blur-md shadow-lg hover:shadow-2xl transition-all
                           duration-300 ease-in-out cursor-pointer"
                        initial={{opacity: 0, x: -30}}
                        whileInView={{opacity: 1, x: 0}}
                        viewport={{once: true, amount: 0.3}}
                        transition={{duration: 0.6}}
                    >
                        <h4 className="text-2xl font-bold mb-4 text-white text-center sm:text-left">
                            For Developers
                        </h4>
                        <ul className="list-disc list-inside text-gray-300 space-y-3 text-left">
                            <li><strong>Convenience:</strong> Find suitable cloud providers faster than you can
                                say &quot;404
                                not found&quot;.
                            </li>
                            <li><strong>Trust:</strong> Obtain legal guarantees and SLAs worth the digital paper
                                they&apos;re
                                written on.
                            </li>
                            <li><strong>No vendor lock-in:</strong> Easy multi-cloud deployments with consistent APIs.
                            </li>
                        </ul>
                    </motion.div>

                    <motion.div
                        className="bg-gradient-to-r from-gray-800/30 to-gray-700/30 border border-white/10
                           p-6 rounded-xl backdrop-blur-md shadow-lg hover:shadow-2xl transition-all
                           duration-300 ease-in-out cursor-pointer"
                        initial={{opacity: 0, x: 30}}
                        whileInView={{opacity: 1, x: 0}}
                        viewport={{once: true, amount: 0.3}}
                        transition={{duration: 0.6, delay: 0.2}}
                    >
                        <h4 className="text-2xl font-bold mb-4 text-white text-center sm:text-left">
                            For Node Providers
                        </h4>
                        <ul className="list-disc list-inside text-gray-300 space-y-3 text-left">
                            <li><strong>Market:</strong> Access to a trillion-dollar crypto market.</li>
                            <li><strong>Users:</strong> Reach a global user base.</li>
                            <li><strong>Fair pricing:</strong> Transparent pricing without a race-to-the-bottom
                                approach.
                            </li>
                        </ul>
                    </motion.div>

                    <motion.div
                        animate={{opacity: 1, y: 0}}
                        initial={{opacity: 0, y: 20}}
                        transition={{duration: 0.5, delay: 0.4}}
                        className="mt-12"
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
                </motion.div>
            </div>
        </section>
    );
};

export default BenefitsSection;
