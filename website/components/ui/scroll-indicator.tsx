import { useEffect, useState } from 'react';
import {AnimatePresence, motion } from 'framer-motion';

const ScrollIndicator = () => {
    const [isVisible, setIsVisible] = useState(true);

    useEffect(() => {
        const handleScroll = () => {
            if (window.scrollY > 50) {
                setIsVisible(false);
            } else {
                setIsVisible(true);
            }
        };

        window.addEventListener('scroll', handleScroll);
        return () => window.removeEventListener('scroll', handleScroll);
    }, []);

    return (
        <AnimatePresence>
            {isVisible && (
                <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 10 }}
                    transition={{ duration: 0.5, ease: 'easeInOut' }}
                    className="absolute bottom-10 transform flex flex-col items-center gap-2"
                >
                    <motion.div
                        animate={{ y: [0, 15, 0] }}
                        transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
                        className="flex flex-col items-center gap-2"
                    >
                        <div className="w-7 h-7 rounded-full border-2 border-white/30 flex items-center justify-center bg-white/10 backdrop-blur-lg shadow-lg animate-pulse">
                            <span className="text-white text-3xl font-light">↓</span>
                        </div>

                        <p className="text-white/80 text-xs uppercase tracking-widest">
                            Scroll Down
                        </p>
                    </motion.div>
                </motion.div>
            )}
        </AnimatePresence>
    );
};

export default ScrollIndicator;
