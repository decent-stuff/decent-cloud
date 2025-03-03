import { motion } from "framer-motion";
import React, { ReactNode } from "react";

interface HeaderSectionProps {
    title: string;
    subtitle?: ReactNode;
}

const HeaderSection: React.FC<HeaderSectionProps> = ({ title, subtitle }) => {
    return (
        <motion.div
            className="text-center mb-12 mt-12"
            initial={{opacity: 0, y: 20}}
            whileInView={{opacity: 1, y: 0}}
            viewport={{once: true, amount: 0.3}} // Trigger when 30% is in viewport
            transition={{duration: 0.8, ease: "easeOut"}}
        >
            <h3 className="text-2xl md:text-3xl font-bold text-center mb-4">
                {title}
            </h3>
            {subtitle && (
                <div className="text-gray-300 max-w-2xl mx-auto text-lg md:text-xl">
                    {subtitle}
                </div>
            )}
        </motion.div>
    );
};

export default HeaderSection;
