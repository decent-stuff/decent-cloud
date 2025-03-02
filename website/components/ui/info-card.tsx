"use client";

import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";

interface InfoSectionProps {
    title: string;
    content: string;
    icon?: string;
}

const InfoCard: React.FC<InfoSectionProps> = ({ title, content, icon }) => {
    const [isExpanded, setIsExpanded] = useState(false);

    return (
        <motion.div
            className="bg-gradient-to-r from-gray-800/30 to-gray-700/30 border border-white/10 p-6 sm:p-8 md:p-10
                   rounded-xl backdrop-blur-md shadow-lg hover:shadow-2xl transition-all duration-300 ease-in-out
                   w-full max-w-md sm:max-w-lg md:max-w-xl mx-auto min-h-[140px] flex flex-col justify-between cursor-pointer"
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
            onClick={() => setIsExpanded(!isExpanded)}
        >
            <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4">
                <div className="flex flex-col sm:flex-row items-center sm:items-start text-center sm:text-left gap-3 w-full">
                    {icon && <span className="text-4xl sm:text-3xl">{icon}</span>}
                    <h4 className="text-lg sm:text-xl md:text-2xl font-bold text-white tracking-wide">
                        {title}
                    </h4>
                </div>
                <button className="text-white/80 hover:text-white transition-all duration-300 text-2xl">
                    {isExpanded ? "−" : "+"}
                </button>
            </div>

            <AnimatePresence>
                {isExpanded && (
                    <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        className="overflow-hidden mt-4"
                    >
                        <div className="text-white/90 leading-relaxed text-sm sm:text-base md:text-lg
                                    text-center sm:text-left transition-all duration-300">
                            <div dangerouslySetInnerHTML={{ __html: content }} />
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        </motion.div>
    );
};

export default InfoCard;
