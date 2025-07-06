use super::common_models::MenKuTen;

pub fn sm_uni_to_jis_mapping(mut state: i32, u: u32) -> (i32, Option<MenKuTen>) {
    loop {
        match state {
            0 => match u {
                230 => break (1, None),
                596 => break (2, None),
                601 => break (3, None),
                602 => break (4, None),
                652 => break (5, None),
                741 => break (6, None),
                745 => break (7, None),
                12363 => break (8, None),
                12365 => break (9, None),
                12367 => break (10, None),
                12369 => break (11, None),
                12371 => break (12, None),
                12459 => break (13, None),
                12461 => break (14, None),
                12463 => break (15, None),
                12465 => break (16, None),
                12467 => break (17, None),
                12475 => break (18, None),
                12484 => break (19, None),
                12488 => break (20, None),
                12791 => break (21, None),
                _ => break (0, None),
            },
            1 => match u {
                768 => break (0, Some(MenKuTen::from(975))),
                _ => {
                    state = 0;
                }
            },
            2 => match u {
                768 => break (0, Some(MenKuTen::from(979))),
                769 => break (0, Some(MenKuTen::from(980))),
                _ => {
                    state = 0;
                }
            },
            3 => match u {
                768 => break (0, Some(MenKuTen::from(983))),
                769 => break (0, Some(MenKuTen::from(984))),
                _ => {
                    state = 0;
                }
            },
            4 => match u {
                768 => break (0, Some(MenKuTen::from(985))),
                769 => break (0, Some(MenKuTen::from(986))),
                _ => {
                    state = 0;
                }
            },
            5 => match u {
                768 => break (0, Some(MenKuTen::from(981))),
                769 => break (0, Some(MenKuTen::from(982))),
                _ => {
                    state = 0;
                }
            },
            6 => match u {
                745 => break (0, Some(MenKuTen::from(1009))),
                _ => {
                    state = 0;
                }
            },
            7 => match u {
                741 => break (0, Some(MenKuTen::from(1008))),
                _ => {
                    state = 0;
                }
            },
            8 => match u {
                12442 => break (0, Some(MenKuTen::from(368))),
                _ => {
                    state = 0;
                }
            },
            9 => match u {
                12442 => break (0, Some(MenKuTen::from(369))),
                _ => {
                    state = 0;
                }
            },
            10 => match u {
                12442 => break (0, Some(MenKuTen::from(370))),
                _ => {
                    state = 0;
                }
            },
            11 => match u {
                12442 => break (0, Some(MenKuTen::from(371))),
                _ => {
                    state = 0;
                }
            },
            12 => match u {
                12442 => break (0, Some(MenKuTen::from(372))),
                _ => {
                    state = 0;
                }
            },
            13 => match u {
                12442 => break (0, Some(MenKuTen::from(462))),
                _ => {
                    state = 0;
                }
            },
            14 => match u {
                12442 => break (0, Some(MenKuTen::from(463))),
                _ => {
                    state = 0;
                }
            },
            15 => match u {
                12442 => break (0, Some(MenKuTen::from(464))),
                _ => {
                    state = 0;
                }
            },
            16 => match u {
                12442 => break (0, Some(MenKuTen::from(465))),
                _ => {
                    state = 0;
                }
            },
            17 => match u {
                12442 => break (0, Some(MenKuTen::from(466))),
                _ => {
                    state = 0;
                }
            },
            18 => match u {
                12442 => break (0, Some(MenKuTen::from(467))),
                _ => {
                    state = 0;
                }
            },
            19 => match u {
                12442 => break (0, Some(MenKuTen::from(468))),
                _ => {
                    state = 0;
                }
            },
            20 => match u {
                12442 => break (0, Some(MenKuTen::from(469))),
                _ => {
                    state = 0;
                }
            },
            21 => match u {
                12442 => break (0, Some(MenKuTen::from(557))),
                _ => {
                    state = 0;
                }
            },
            _ => {}
        }
    }
}
